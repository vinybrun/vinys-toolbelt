/* Nebula Market — cart, toasts, forms */
const PRODUCTS = [
  { id: "aurora-lamp", name: "Aurora Desk Lamp", price: 79, category: "home", emoji: "💡", badge: "new", blurb: "Soft gradient lighting with touch dimming." },
  { id: "orbit-mug", name: "Orbit Ceramic Mug", price: 24, category: "kitchen", emoji: "☕", badge: null, blurb: "Double-wall ceramic that keeps drinks warm." },
  { id: "nova-notebook", name: "Nova Dot Notebook", price: 18, category: "stationery", emoji: "📓", badge: "sale", blurb: "A5 dotted pages, lay-flat binding." },
  { id: "pulse-headphones", name: "Pulse Headphones", price: 149, category: "audio", emoji: "🎧", badge: "new", blurb: "ANC on-ear with 32h battery." },
  { id: "comet-bottle", name: "Comet Water Bottle", price: 32, category: "outdoor", emoji: "🧴", badge: null, blurb: "Insulated stainless steel, 750ml." },
  { id: "lunar-wallet", name: "Lunar Slim Wallet", price: 45, category: "accessories", emoji: "👛", badge: "sale", blurb: "RFID-safe, holds 8 cards." },
  { id: "stardust-candle", name: "Stardust Candle", price: 28, category: "home", emoji: "🕯️", badge: null, blurb: "Cedar & bergamot, 40-hour burn." },
  { id: "galaxy-poster", name: "Galaxy Art Print", price: 36, category: "decor", emoji: "🖼️", badge: null, blurb: "Museum-quality 18×24 print." },
  { id: "meteor-keyboard", name: "Meteor Mechanical KB", price: 129, category: "tech", emoji: "⌨️", badge: "new", blurb: "Hot-swap switches, RGB glow." },
];

const STORAGE_KEY = "nebula_cart_v1";
const SESSION_KEY = "nebula_user_v1";

function money(n) {
  return `$${Number(n).toFixed(2)}`;
}

function getCart() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
  } catch {
    return [];
  }
}

function setCart(items) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  updateCartCount();
}

function cartCount() {
  return getCart().reduce((s, i) => s + i.qty, 0);
}

function updateCartCount() {
  document.querySelectorAll("[data-cart-count]").forEach((el) => {
    el.textContent = String(cartCount());
  });
}

function addToCart(productId, qty = 1) {
  const product = PRODUCTS.find((p) => p.id === productId);
  if (!product) return;
  const cart = getCart();
  const existing = cart.find((i) => i.id === productId);
  if (existing) existing.qty += qty;
  else cart.push({ id: productId, qty });
  setCart(cart);
  toast(`Added ${product.name}`, "success");
}

function removeFromCart(productId) {
  setCart(getCart().filter((i) => i.id !== productId));
  toast("Removed from cart", "success");
}

function setQty(productId, qty) {
  const cart = getCart();
  const item = cart.find((i) => i.id === productId);
  if (!item) return;
  item.qty = Math.max(1, qty);
  setCart(cart);
}

function toast(message, kind = "success") {
  const root = document.getElementById("toast-root") || createToastRoot();
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.setAttribute("role", "status");
  el.textContent = message;
  root.appendChild(el);
  setTimeout(() => el.remove(), 2800);
}

function createToastRoot() {
  const root = document.createElement("div");
  root.id = "toast-root";
  document.body.appendChild(root);
  return root;
}

function productById(id) {
  return PRODUCTS.find((p) => p.id === id);
}

function renderProductCard(p) {
  const badge = p.badge
    ? `<span class="badge badge-${p.badge}">${p.badge}</span>`
    : "";
  return `
    <article class="card" data-product-id="${p.id}">
      <div class="card-media" aria-hidden="true">${p.emoji2 || p.emoji}</div>
      <div class="card-body">
        <div class="tag-row">${badge}<span class="tag">${p.category}</span></div>
        <h3><a href="product.html?id=${p.id}">${p.name}</a></h3>
        <p class="muted" style="margin:.25rem 0 .6rem;font-size:.9rem">${p.blurb}</p>
        <div style="display:flex;justify-content:space-between;align-items:center;gap:.5rem">
          <span class="price">${money(p.price)}</span>
          <button class="btn btn-primary btn-sm" type="button" data-add="${p.id}" aria-label="Add ${p.name} to cart">Add</button>
        </div>
      </div>
    </article>`;
}

function wireAddButtons(root = document) {
  root.querySelectorAll("[data-add]").forEach((btn) => {
    btn.addEventListener("click", () => addToCart(btn.getAttribute("data-add")));
  });
}

function setActiveNav() {
  const path = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".nav-links a[href]").forEach((a) => {
    const href = a.getAttribute("href");
    if (href === path || (path === "" && href === "index.html")) {
      a.classList.add("active");
      a.setAttribute("aria-current", "page");
    }
  });
}

function getUser() {
  try {
    return JSON.parse(sessionStorage.getItem(SESSION_KEY) || "null");
  } catch {
    return null;
  }
}

function setUser(user) {
  if (user) sessionStorage.setItem(SESSION_KEY, JSON.stringify(user));
  else sessionStorage.removeItem(SESSION_KEY);
}

function validateEmail(email) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}

// page bootstraps
document.addEventListener("DOMContentLoaded", () => {
  updateCartCount();
  setActiveNav();
  createToastRoot();

  const page = document.body.dataset.page;
  if (page === "home") initHome();
  if (page === "shop") initShop();
  if (page === "product") initProduct();
  if (page === "cart") initCart();
  if (page === "checkout") initCheckout();
  if (page === "login") initLogin();
  if (page === "contact") initContact();
});

function initHome() {
  const featured = document.getElementById("featured-products");
  if (!featured) return;
  featured.innerHTML = PRODUCTS.slice(0, 3).map(renderProductCard).join("");
  wireAddButtons(featured);
}

function initShop() {
  const grid = document.getElementById("product-grid");
  const search = document.getElementById("search");
  const cat = document.getElementById("category-filter");
  const empty = document.getElementById("shop-empty");
  const countEl = document.getElementById("result-count");

  function render() {
    const q = (search?.value || "").toLowerCase().trim();
    const c = cat?.value || "all";
    const list = PRODUCTS.filter((p) => {
      const matchQ =
        !q ||
        p.name.toLowerCase().includes(q) ||
        p.blurb.toLowerCase().includes(q) ||
        p.category.includes(q);
      const matchC = c === "all" || p.category === c;
      return matchQ && matchC;
    });
    if (countEl) countEl.textContent = `${list.length} product${list.length === 1 ? "" : "s"}`;
    if (!list.length) {
      grid.innerHTML = "";
      if (empty) empty.hidden = false;
      return;
    }
    if (empty) empty.hidden = true;
    grid.innerHTML = list.map(renderProductCard).join("");
    wireAddButtons(grid);
  }

  search?.addEventListener("input", render);
  cat?.addEventListener("change", render);
  render();
}

function initProduct() {
  const params = new URLSearchParams(location.search);
  const id = params.get("id") || "aurora-lamp";
  const p = productById(id) || PRODUCTS[0];
  document.getElementById("p-name").textContent = p.name;
  document.getElementById("p-price").textContent = money(p.price);
  document.getElementById("p-blurb").textContent = p.blurb;
  document.getElementById("p-category").textContent = p.category;
  document.getElementById("p-emoji").textContent = p.emoji2 || p.emoji;
  document.title = `${p.name} · Nebula Market`;
  document.getElementById("add-detail").addEventListener("click", () => {
    const qty = parseInt(document.getElementById("detail-qty").value || "1", 10);
    addToCart(p.id, qty);
  });
}

function initCart() {
  const body = document.getElementById("cart-body");
  const summary = document.getElementById("cart-summary");
  const empty = document.getElementById("cart-empty");
  const tableWrap = document.getElementById("cart-table-wrap");

  function render() {
    const cart = getCart();
    if (!cart.length) {
      empty.hidden = false;
      tableWrap.hidden = true;
      summary.innerHTML = "";
      return;
    }
    empty.hidden = true;
    tableWrap.hidden = false;
    let subtotal = 0;
    body.innerHTML = cart
      .map((item) => {
        const p = productById(item.id);
        if (!p) return "";
        const line = p.price * item.qty;
        subtotal += line;
        return `<tr data-line="${p.id}">
          <td>${p.emoji2 || p.emoji} <strong>${p.name}</strong></td>
          <td>${money(p.price)}</td>
          <td>
            <div class="qty">
              <button type="button" data-dec="${p.id}" aria-label="Decrease quantity">−</button>
              <span data-qty="${p.id}">${item.qty}</span>
              <button type="button" data-inc="${p.id}" aria-label="Increase quantity">+</button>
            </div>
          </td>
          <td>${money(line)}</td>
          <td><button class="btn btn-sm btn-secondary" type="button" data-remove="${p.id}">Remove</button></td>
        </tr>`;
      })
      .join("");

    const shipping = subtotal >= 100 ? 0 : 8;
    const total = subtotal + shipping;
    summary.innerHTML = `
      <div class="panel">
        <h2 style="margin-top:0">Order summary</h2>
        <p class="muted">Subtotal: <strong id="subtotal">${money(subtotal)}</strong></p>
        <p class="muted">Shipping: <strong id="shipping">${shipping === 0 ? "Free" : money(shipping)}</strong></p>
        <p>Total: <strong class="price" id="grand-total">${money(total)}</strong></p>
        <a class="btn btn-primary" href="checkout.html" id="checkout-link" style="width:100%;justify-content:center;margin-top:.5rem">Checkout</a>
      </div>`;

    body.querySelectorAll("[data-inc]").forEach((b) =>
      b.addEventListener("click", () => {
        const id = b.getAttribute("data-inc");
        const item = getCart().find((i) => i.id === id);
        setQty(id, (item?.qty || 1) + 1);
        render();
      })
    );
    body.querySelectorAll("[data-dec]").forEach((b) =>
      b.addEventListener("click", () => {
        const id = b.getAttribute("data-dec");
        const item = getCart().find((i) => i.id === id);
        setQty(id, Math.max(1, (item?.qty || 1) - 1));
        render();
      })
    );
    body.querySelectorAll("[data-remove]").forEach((b) =>
      b.addEventListener("click", () => {
        removeFromCart(b.getAttribute("data-remove"));
        render();
      })
    );
  }
  render();
}

function initCheckout() {
  const form = document.getElementById("checkout-form");
  const banner = document.getElementById("order-success");
  const cart = getCart();
  if (!cart.length) {
    document.getElementById("checkout-empty").hidden = false;
    form.hidden = true;
    return;
  }
  const subtotal = cart.reduce((s, i) => {
    const p = productById(i.id);
    return s + (p ? p.price * i.qty : 0);
  }, 0);
  document.getElementById("checkout-subtotal").textContent = money(subtotal);
  document.getElementById("checkout-total").textContent = money(subtotal + (subtotal >= 100 ? 0 : 8));

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    let ok = true;
    form.querySelectorAll(".field").forEach((f) => f.classList.remove("invalid"));
    const name = form.elements.namedItem("fullName");
    const email = form.elements.namedItem("email");
    const address = form.elements.namedItem("address");
    const city = form.elements.namedItem("city");
    if (!name.value.trim()) {
      name.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!validateEmail(email.value)) {
      email.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!address.value.trim()) {
      address.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!city.value.trim()) {
      city.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!ok) {
      toast("Please fix the highlighted fields", "error");
      return;
    }
    const orderId = "NB-" + Math.random().toString(36).slice(2, 8).toUpperCase();
    setCart([]);
    form.hidden = true;
    banner.classList.add("show");
    banner.hidden = false;
    document.getElementById("order-id").textContent = orderId;
    document.getElementById("order-email").textContent = email.value;
    toast("Order placed!", "success");
  });
}

function initLogin() {
  const form = document.getElementById("login-form");
  const status = document.getElementById("login-status");
  const user = getUser();
  if (user) {
    status.hidden = false;
    status.textContent = `Signed in as ${user.email}`;
    form.hidden = true;
    return;
  }
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    form.querySelectorAll(".field").forEach((f) => f.classList.remove("invalid"));
    const email = form.elements.namedItem("email");
    const password = form.elements.namedItem("password");
    let ok = true;
    if (!validateEmail(email.value)) {
      email.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!password.value || password.value.length < 6) {
      password.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!ok) {
      toast("Check email and password", "error");
      return;
    }
    // demo credentials
    if (email.value === "demo@nebula.test" && password.value === "nebula1") {
      setUser({ email: email.value, name: "Demo Voyager" });
      toast("Welcome back!", "success");
      status.hidden = false;
      status.textContent = `Signed in as ${email.value}`;
      form.hidden = true;
      document.getElementById("login-success").classList.add("show");
      document.getElementById("login-success").hidden = false;
    } else {
      toast("Invalid credentials (try demo@nebula.test / nebula1)", "error");
      document.getElementById("login-error").hidden = false;
    }
  });
}

function initContact() {
  const form = document.getElementById("contact-form");
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    form.querySelectorAll(".field").forEach((f) => f.classList.remove("invalid"));
    const name = form.elements.namedItem("name");
    const email = form.elements.namedItem("email");
    const message = form.elements.namedItem("message");
    let ok = true;
    if (!name.value.trim()) {
      name.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!validateEmail(email.value)) {
      email.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!message.value.trim() || message.value.trim().length < 10) {
      message.closest(".field").classList.add("invalid");
      ok = false;
    }
    if (!ok) {
      toast("Please complete the form", "error");
      return;
    }
    form.hidden = true;
    document.getElementById("contact-success").classList.add("show");
    document.getElementById("contact-success").hidden = false;
    toast("Message sent", "success");
  });
}

// export for debugging in console
window.Nebula = { PRODUCTS, getCart, addToCart, getUser };
