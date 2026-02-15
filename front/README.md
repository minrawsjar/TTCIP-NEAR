# 🌐 Front — Landing Page & dApp

> **Vite + Vanilla JS + GSAP**

Marketing landing page and wallet dApp interface for Text-to-Chain. Features smooth scroll animations, wallet connection, and a dashboard for interacting with the platform.

---

## Pages

| File | Purpose |
|------|---------|
| `index.html` | Landing page — hero, features, how-it-works |
| `dapp.html` | Wallet dApp — connect, send, swap, balance |

---

## Folder Structure

```
front/
├── index.html              # Landing page
├── dapp.html               # dApp interface
├── dapp.js                 # dApp logic (wallet connect, transactions)
├── script.js               # Landing page animations
├── styles.css              # Landing page styles
├── package.json            # Dependencies (Vite, GSAP, Lenis, viem)
├── vite.config.js          # Vite dev server config
└── elements/
    ├── dapp.css            # dApp styles
    ├── dash.css            # Dashboard styles
    ├── dash.js             # Dashboard logic
    ├── pools.css           # Pool management styles
    ├── pools.js            # Pool interaction logic
    └── wallet-config.js    # Wallet/chain configuration
```

---

## Tech Stack

| Technology | Purpose |
|------------|---------|
| **Vite** | Dev server + bundler |
| **GSAP** | Scroll animations |
| **Lenis** | Smooth scrolling |
| **viem** | Ethereum client |
| **Wagmi Core** | Wallet connection |

---

## Setup

```bash
cd front
npm install
npm run dev
```

Opens at [http://localhost:5173](http://localhost:5173).

