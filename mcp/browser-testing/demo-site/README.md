# Nebula Market — demo storefront

Rich multi-page static site used to exercise the **browser testing MCP** and **full-mcp-client**.

## Pages

| Page | Path | Features |
|------|------|----------|
| Home | `index.html` | Hero, featured products, CTAs |
| Shop | `shop.html` | Search, category filter, add-to-cart |
| Product | `product.html?id=` | Detail, quantity, add-to-cart |
| Cart | `cart.html` | Qty controls, summary, checkout link |
| Checkout | `checkout.html` | Validated form, order confirmation |
| Account | `login.html` | Demo auth (`demo@nebula.test` / `nebula1`) |
| Contact | `contact.html` | Message form with validation |

## Serve locally

```bash
cd examples/servers/demo-site
python3 -m http.server 8877 --bind 127.0.0.1
```

## Test via MCP

```bash
# from repo root
cargo run -p full-mcp-client -- --preset browser script \
  examples/servers/mcp-scripts/nebula-e2e.json
```

Suites: `browser-tests/nebula-*.yaml`
