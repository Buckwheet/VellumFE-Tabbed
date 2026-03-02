#!/usr/bin/env python3
"""
eAccess authentication test tool for VellumFE-Tabbed.
Tests the full eAccess flow: K -> A -> G -> C -> L
Run: python eaccess_test.py
"""

import ssl
import socket
import sys

HOST = "eaccess.play.net"
PORT = 7910


def connect():
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    raw = socket.create_connection((HOST, PORT), timeout=10)
    return ctx.wrap_socket(raw)


def send(s, line):
    msg = (line + "\n").encode("windows-1252")
    print(f"  SEND: {repr(line)}")
    s.sendall(msg)


def recv(s):
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(4096)
        if not chunk:
            break
        buf += chunk
    resp = buf.decode("windows-1252").rstrip("\r\n")
    print(f"  RECV: {repr(resp)}")
    return resp


def obfuscate(password, hash_key):
    result = []
    for p, h in zip(password.encode("windows-1252"), hash_key.encode("windows-1252")):
        result.append(((p - 32) ^ h) + 32)
    return bytes(result)


def run(account, password, game_code, character):
    print(f"\n=== eAccess Test ===")
    print(f"Account:   {account}")
    print(f"Game:      {game_code}")
    print(f"Character: {character}")
    print(f"Connecting to {HOST}:{PORT}...\n")

    s = connect()
    print("TLS connected\n")

    # K - get hash
    print("[Step 1] K - request password hash")
    send(s, "K")
    hash_key = recv(s)

    # A - login
    print("\n[Step 2] A - authenticate")
    enc = obfuscate(password, hash_key)
    payload = b"A\t" + account.encode("windows-1252") + b"\t" + enc + b"\n"
    print(f"  SEND: A\\t{account}\\t<encoded_password>")
    s.sendall(payload)
    auth_resp = recv(s)
    tokens = auth_resp.split("\t")
    if len(tokens) < 3:
        print(f"\nFAIL: Unexpected auth response: {auth_resp}")
        return
    status = tokens[2]
    if status in ("PASSWORD", "REJECT", "NORECORD"):
        print(f"\nFAIL: Auth rejected — {status}")
        return
    if tokens[1].lower() != account.lower():
        print(f"\nFAIL: Username mismatch — got '{tokens[1]}'")
        return
    print("  AUTH OK")

    # G - select game
    print(f"\n[Step 3] G - select game {game_code}")
    send(s, f"G\t{game_code}")
    g_resp = recv(s)

    # C - list characters
    print("\n[Step 4] C - list characters")
    send(s, "C")
    c_resp = recv(s)
    c_tokens = c_resp.split("\t")
    # Format: C\t?\t?\t?\t?\tCODE\tNAME\tCODE\tNAME...
    char_code = None
    if len(c_tokens) > 5:
        i = 5
        print("  Characters found:")
        while i + 1 < len(c_tokens):
            code = c_tokens[i]
            name = c_tokens[i + 1]
            print(f"    {name} ({code})")
            if name.lower() == character.lower():
                char_code = code
            i += 2
    if not char_code:
        print(f"\nFAIL: Character '{character}' not found in list")
        return
    print(f"  Selected: {character} = {char_code}")

    # L - launch
    print(f"\n[Step 5] L - launch {character}")
    send(s, f"L\t{char_code}\tSTORM")
    l_resp = recv(s)
    l_tokens = l_resp.split("\t")
    if len(l_tokens) < 2 or l_tokens[0] != "L":
        print(f"\nFAIL: Unexpected launch response: {l_resp}")
        return
    if l_tokens[1] != "OK":
        print(f"\nFAIL: Launch rejected — {l_tokens[1]}")
        if l_tokens[1] == "PROBLEM":
            print("  PROBLEM = account subscription issue or character already logged in")
        return

    # Parse key/host/port
    props = {}
    for token in l_tokens[2:]:
        if "=" in token:
            k, v = token.split("=", 1)
            props[k] = v
    print(f"\nSUCCESS!")
    print(f"  Game host: {props.get('GAMEHOST', '?')}")
    print(f"  Game port: {props.get('GAMEPORT', '?')}")
    print(f"  Key:       {props.get('KEY', '?')[:8]}...")

    s.close()


if __name__ == "__main__":
    import getpass

    account = input("Account name: ").strip()
    password = getpass.getpass("Password: ")
    game_code = input("Game code [GS4]: ").strip() or "GS4"
    character = input("Character name [Brashka]: ").strip() or "Brashka"

    try:
        run(account, password, game_code, character)
    except Exception as e:
        print(f"\nERROR: {e}")
        sys.exit(1)
