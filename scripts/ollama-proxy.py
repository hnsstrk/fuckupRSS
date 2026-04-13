#!/usr/bin/python3
"""
TCP-Proxy fuer Ollama-Server im lokalen Netzwerk.

Workaround fuer macOS Tahoe 26.x: Ad-hoc-signierte Binaries (Rust/Tauri
Debug-Builds) koennen nicht auf LAN-IPs zugreifen. Dieses Script laeuft
unter /usr/bin/python3 (Apple-signiert) und leitet den Traffic weiter.

Usage:
    /usr/bin/python3 scripts/ollama-proxy.py                          # Default: 11435 -> <remote-host>:11434
    /usr/bin/python3 scripts/ollama-proxy.py --remote 10.0.0.5        # Anderer Remote-Host
    /usr/bin/python3 scripts/ollama-proxy.py --local-port 11440       # Anderer lokaler Port
    /usr/bin/python3 scripts/ollama-proxy.py --remote-port 8080       # Anderer Remote-Port

Dann in fuckupRSS Settings die Ollama-URL auf http://localhost:11435 setzen.
"""
import argparse
import select
import socket
import signal
import sys
import threading
import time

DEFAULT_REMOTE_HOST = "localhost"
DEFAULT_REMOTE_PORT = 11434
DEFAULT_LOCAL_PORT = 11435
BUFFER_SIZE = 65536

active_connections = 0
total_connections = 0
lock = threading.Lock()


def forward_data(src, dst, name):
    try:
        while True:
            ready, _, _ = select.select([src], [], [], 30)
            if not ready:
                continue
            data = src.recv(BUFFER_SIZE)
            if not data:
                break
            dst.sendall(data)
    except (OSError, BrokenPipeError, ConnectionResetError):
        pass
    finally:
        try:
            src.shutdown(socket.SHUT_RD)
        except OSError:
            pass
        try:
            dst.shutdown(socket.SHUT_WR)
        except OSError:
            pass


def handle_client(client_sock, remote_host, remote_port):
    global active_connections, total_connections
    with lock:
        active_connections += 1
        total_connections += 1

    try:
        remote_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        remote_sock.settimeout(10)
        remote_sock.connect((remote_host, remote_port))
        remote_sock.settimeout(None)
    except Exception as e:
        print(f"  Verbindung zu {remote_host}:{remote_port} fehlgeschlagen: {e}")
        client_sock.close()
        with lock:
            active_connections -= 1
        return

    t1 = threading.Thread(
        target=forward_data, args=(client_sock, remote_sock, "client->remote"), daemon=True
    )
    t2 = threading.Thread(
        target=forward_data, args=(remote_sock, client_sock, "remote->client"), daemon=True
    )
    t1.start()
    t2.start()
    t1.join()
    t2.join()

    client_sock.close()
    remote_sock.close()

    with lock:
        active_connections -= 1


def main():
    parser = argparse.ArgumentParser(description="Ollama LAN Proxy")
    parser.add_argument("--remote", default=DEFAULT_REMOTE_HOST, help="Remote Ollama host")
    parser.add_argument("--remote-port", type=int, default=DEFAULT_REMOTE_PORT)
    parser.add_argument("--local-port", type=int, default=DEFAULT_LOCAL_PORT)
    args = parser.parse_args()

    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

    try:
        server.bind(("127.0.0.1", args.local_port))
    except OSError as e:
        print(f"Port {args.local_port} belegt: {e}")
        sys.exit(1)

    server.listen(32)
    server.settimeout(1)

    print(f"Ollama-Proxy gestartet: localhost:{args.local_port} -> {args.remote}:{args.remote_port}")
    print(f"Setze Ollama-URL in fuckupRSS auf: http://localhost:{args.local_port}")
    print("Ctrl+C zum Beenden\n")

    def shutdown(sig, frame):
        print(f"\nProxy beendet. {total_connections} Verbindungen verarbeitet.")
        server.close()
        sys.exit(0)

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    while True:
        try:
            client, addr = server.accept()
            threading.Thread(
                target=handle_client,
                args=(client, args.remote, args.remote_port),
                daemon=True,
            ).start()
        except socket.timeout:
            continue
        except OSError:
            break


if __name__ == "__main__":
    main()
