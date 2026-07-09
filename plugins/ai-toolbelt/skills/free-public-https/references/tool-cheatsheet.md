# Tool cheatsheet — free public exposure

## Serveo

```bash
ssh -o StrictHostKeyChecking=accept-new \
    -o ServerAliveInterval=30 \
    -o ExitOnForwardFailure=yes \
    -R 80:127.0.0.1:8080 \
    serveo.net
```

- URL host pattern: `*.serveousercontent.com`
- Typical cert: ZeroSSL (public CA)
- One SSH session ≈ one local HTTP origin

## cloudflared quick tunnel

```bash
cloudflared tunnel --url http://127.0.0.1:8080 --no-autoupdate
cloudflared tunnel --url http://127.0.0.1:8080 --protocol http2 --no-autoupdate
```

- URL host pattern: `*.trycloudflare.com`
- No account for quick tunnels; ephemeral
- DNS issues: try `dig @1.1.1.1` or use Serveo instead

## bore

```bash
bore local 8080 --to bore.pub
bore server   # self-host control plane
bore local 8080 --to <SERVER_IP> --port 40050
```

- Public TCP: `bore.pub:<port>`
- Not HTTPS by itself

## Optional free IP→hostname (no signup)

Given IP `A.B.C.D`:

- `A-B-C-D.nip.io`
- `A.B.C.D.sslip.io`

## Local TLS (self-signed)

```bash
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout key.pem -out cert.pem -days 30 \
  -subj "/CN=localhost"
```

Browser will warn; fine for labs only.

## Health checks

```bash
curl -sS http://127.0.0.1:PORT/health
curl -sS -m 15 -w "http=%{http_code} tls=%{ssl_verify_result}\n" \
  -o /dev/null https://PUBLIC_HOST/health
```
