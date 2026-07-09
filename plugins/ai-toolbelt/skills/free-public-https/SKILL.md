---
name: free-public-https
description: >
  Deploy a local HTTP/TCP service to a public URL with free HTTPS (no cloud account)
  using Serveo SSH tunnels, Cloudflare quick tunnels (trycloudflare.com), or bore.pub
  for raw TCP. Use when the user wants free public HTTPS, expose localhost, tunnel
  a lab/demo without a VPS, Serveo, cloudflared, bore.pub, or runs /free-public-https.
---

# Free public HTTPS / TCP tunnels (no VPS account)

## Goal

Publish a process listening on `localhost` so others (or other processes) can reach it on the **public internet**, preferably with a **trusted TLS certificate** (browser padlock, no self-signed warning).

This skill captures a battle-tested pattern from the `device-net` lab: **run services locally, publish them with free reverse tunnels**.

## Be precise in language

- Say **simulated services / processes**, not “emulated devices,” unless you are actually using QEMU/netns/hardware emulation.
- Say **public reverse tunnel**, not “we deployed to a free VPS,” unless the user owns a VM.
- Free tunnels are **third-party middleboxes**. Assume operators can see plaintext after TLS termination at their edge (and always see metadata).

## Decision tree

| Need | Prefer | Notes |
|------|--------|--------|
| **HTTPS + real CA cert + no account** | **Serveo** or **cloudflared quick tunnel** | Best for dashboards and HTTPS APIs |
| **Raw TCP** (custom protocols, not HTTP) | **bore.pub** | Port is random; not browser HTTPS by itself |
| **Self-hosted relay you control** | `bore server` on a VPS | Needs a host with open ports / account |
| **Avoid public third parties** | localhost / netns / private VPN | No free public URL |

Default for “I need a public https://… URL now”: **Serveo**, fallback **cloudflared**.

---

## Prerequisites

1. Service healthy **locally** first:
   ```bash
   curl -sS http://127.0.0.1:8080/health
   ```
2. Outbound SSH and/or HTTPS allowed from the environment.
3. For Serveo: `ssh` client. For Cloudflare: `cloudflared` binary. For bore: `bore` binary.

Do **not** open inbound firewall ports on the lab host if tunnels work.

---

## Method A — Serveo (SSH reverse tunnel → free HTTPS)

### How it works

```text
Internet client
   --HTTPS:443--> serveousercontent.com (public CA cert, e.g. ZeroSSL)
                      |
                      | SSH reverse forward
                      v
               127.0.0.1:LOCAL_PORT  (your process)
```

### Steps

1. Run the app on a fixed local port (e.g. `8080`).
2. Publish:
   ```bash
   ssh -o StrictHostKeyChecking=accept-new \
       -o ServerAliveInterval=30 \
       -o ExitOnForwardFailure=yes \
       -R 80:127.0.0.1:8080 \
       serveo.net
   ```
3. Parse the log line:
   ```text
   Forwarding HTTP traffic from https://<id>-<something>.serveousercontent.com
   ```
4. Verify:
   ```bash
   curl -sS -m 15 "https://<that-host>/health"
   # Expect HTTP 200 and ssl verify OK
   ```

### Two services (gateway + history pattern)

Serveo maps **one remote HTTP front door per SSH session**. For two local ports, run **two** SSH processes:

```bash
# Terminal / process 1 — app A on :8080
ssh -o ServerAliveInterval=30 -o ExitOnForwardFailure=yes \
  -R 80:127.0.0.1:8080 serveo.net > serveo-a.log 2>&1 &

# Terminal / process 2 — app B on :9090
ssh -o ServerAliveInterval=30 -o ExitOnForwardFailure=yes \
  -R 80:127.0.0.1:9090 serveo.net > serveo-b.log 2>&1 &
```

You get **two different** `https://…serveousercontent.com` hostnames. Point clients at the correct one (`GATEWAY_URL`, `HISTORY_URL`, etc.).

### Serveo pitfalls

- If local app dies, logs show `connect_to 127.0.0.1 port N: failed` — restart the app, then often the SSH session.
- Hostname is **ephemeral**; restart SSH ⇒ new URL ⇒ update all clients.
- Pseudo-TTY warnings (`Pseudo-terminal will not be allocated`) are normal when stdin is not a TTY.
- Do not use `-N` if you need the URL printed (or parse differently); without remote command, still works with `-R` when Serveo prints the forward line on connect.

---

## Method B — Cloudflare quick tunnel (free HTTPS, real CA)

### How it works

```text
Browser --HTTPS--> *.trycloudflare.com (Cloudflare / public CA)
                      |
                      | cloudflared
                      v
               http://127.0.0.1:PORT
```

### Steps

1. Install binary (example):
   ```bash
   curl -sSL -o cloudflared \
     https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
   chmod +x cloudflared
   ```
2. Publish:
   ```bash
   ./cloudflared tunnel --url http://127.0.0.1:8080 --no-autoupdate
   ```
3. Capture:
   ```text
   https://random-words.trycloudflare.com
   ```
4. Verify TLS:
   ```bash
   curl -sS -o /dev/null -w "%{http_code} ssl=%{ssl_verify_result}\n" \
     https://random-words.trycloudflare.com/health
   # ssl_verify_result 0 = OK
   ```

### Cloudflare pitfalls

- **DNS failure (“Server Not Found”)** for `*.trycloudflare.com` is common on some resolvers/networks.  
  - Diagnose: `dig +short HOST @1.1.1.1`  
  - If only broken resolvers fail, tell the user to try another DNS or use **Serveo** instead.
- Account-less tunnels have **no uptime SLA**; URL changes when the process restarts.
- Prefer **http2** if QUIC is flaky:  
  `cloudflared tunnel --url http://127.0.0.1:8080 --protocol http2`

---

## Method C — bore.pub (free public TCP)

### How it works

```text
Remote client --TCP--> bore.pub:RANDOM_PORT
                          |
                          | bore local
                          v
                   127.0.0.1:LOCAL_PORT
```

### Steps

1. Get `bore` (GitHub releases or `cargo install bore-cli`).
2. ```bash
   bore local 9000 --to bore.pub
   # listening at bore.pub:41221
   ```
3. Clients use `bore.pub:41221` (hostname is already DNS — not only an IP).

### Optional free DNS aliases for the IP

If you resolve `bore.pub` → `A.B.C.D`, these work without an account:

- `A-B-C-D.nip.io:PORT`
- `A.B.C.D.sslip.io:PORT`

### bore + HTTPS

- **bore alone is not browser HTTPS.**
- Pattern for “HTTPS on bore”: local TLS terminator (e.g. Python/OpenSSL reverse proxy on 8443) + `bore local 8443 --to bore.pub` → `https://bore.pub:PORT` with **self-signed** cert (browser warning).
- Prefer Serveo/cloudflared when you want a **trusted** cert without warnings.

### Self-hosted bore (still free of SaaS, needs a host)

```bash
# on a machine with a reachable IP
bore server
# clients: bore local 8080 --to <that-ip>
```

---

## Method D — Self-signed local TLS (lab only)

When you only need encryption and can click through warnings:

1. `openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 30`
2. HTTPS reverse-proxy to the plain HTTP app (or native TLS in the app).
3. Publish 8443 via bore or leave on localhost.

Never present this as “secure production HTTPS.”

---

## Multi-service lab checklist (device-net style)

Use this when several local processes must look like separate public servers:

1. **Prove local health** for each port (`8080` gateway, `9090` history, …).
2. **One public tunnel per HTTP service** (two Serveo sessions or two cloudflared processes).
3. **Configure clients with public base URLs**, not `127.0.0.1`, if you claim multi-machine topology:
   ```bash
   GATEWAY_URL=https://…serveousercontent.com
   HISTORY_URL=https://…other….serveousercontent.com
   ```
4. **Persist URLs** to files (`logs/dashboard-https-secure.url`, `logs/history-https.url`) and a `PUBLIC.txt` summary.
5. **Restart order**: local apps → tunnels → clients (sensors/actuators).
6. **If tunnel says connect_to failed**: local process died or bound the wrong interface (`0.0.0.0` vs `127.0.0.1` is fine; missing process is not).

### What not to do

- Do not call process simulation “MCU emulation.”
- Do not put `HISTORY_URL=http://127.0.0.1:9090` while advertising public multi-site sensors — use a public history URL.
- Do not send secrets through free tunnels.
- Do not assume free SOCKS5 proxies are safe or stable (optional geo hop only).

---

## Verification commands (copy/paste)

```bash
# Local
curl -sS -m 3 http://127.0.0.1:8080/health

# Public HTTPS (Serveo / CF)
curl -sS -m 15 -o /dev/null -w "http=%{http_code} tls_verify=%{ssl_verify_result}\n" \
  "https://YOUR_HOST/health"

# Certificate issuer (expect public CA, not "device-net-lab")
echo | openssl s_client -connect YOUR_HOST:443 -servername YOUR_HOST 2>/dev/null \
  | openssl x509 -noout -issuer -subject

# DNS debug for trycloudflare
dig +short YOUR_HOST @1.1.1.1
```

`tls_verify=0` from curl means verification succeeded.

---

## Security & ethics blurb (always mention once)

Free public tunnels are excellent for **demos and labs**. They are **not** a substitute for:

- Your own domain + Let’s Encrypt on infrastructure you control  
- Authentication / authorization on the app  
- Confidential data handling  

Lab traffic should be synthetic or non-sensitive.

---

## Agent workflow summary

When asked to “get a free public HTTPS URL for this local server”:

1. Confirm local listen + health.
2. Try **Serveo** first; parse URL; `curl` verify TLS.
3. If Serveo fails, try **cloudflared** quick tunnel; if user hits “Server Not Found”, switch DNS or fall back to Serveo.
4. For non-HTTP, use **bore.pub**.
5. For multiple services, multiple tunnels + explicit env URLs.
6. Write URLs to disk; restart dependents; speak accurately (tunnel ≠ VPS, process ≠ emulated MCU).

## Reference

See `references/tool-cheatsheet.md` in this skill directory for flag cheat sheets.
