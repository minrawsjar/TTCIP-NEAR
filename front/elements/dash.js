const TOKENIZED_ASSETS = [
  { currency: "Naira", code: "NGN", country: "Nigeria", active: true, tvl: "1.2M" },
  { currency: "Rupee", code: "INR", country: "India", active: false },
  { currency: "Shilling", code: "KES", country: "Kenya", active: false },
  { currency: "Real", code: "BRL", country: "Brazil", active: false },
  { currency: "CFA Franc", code: "XOF", country: "Senegal", active: false },
  { currency: "Cedi", code: "GHS", country: "Ghana", active: false },
];

function halfTvl(tvl) {
  if (!tvl || tvl === "—") return null;
  const match = tvl.match(/^([\d.]+)(M|K)?$/i);
  if (!match) return null;
  const num = parseFloat(match[1]) / 2;
  const suffix = (match[2] || "").toUpperCase();
  return num + (suffix ? suffix : "");
}

const YIELD_OPPORTUNITIES = [
  { pool: "Nigeria · MTN", currency: "NGN", apy: "25%", tvl: "1.2M", active: true },
  { pool: "India · Airtel", currency: "INR", apy: "—", tvl: "—", active: false },
  { pool: "Kenya · Safaricom", currency: "KES", apy: "—", tvl: "—", active: false },
  { pool: "Brazil · Claro", currency: "BRL", apy: "—", tvl: "—", active: false },
];

/** Tokenized tab: overview text + tokenized assets as cards */
export function renderTokenizedContent() {
  return `
    <div class="dash-panel dash-panel--tokenized">
      <div class="dash-overview">
        <h2 class="dash-overview__title">Overview</h2>
        <p class="dash-overview__text">Protocol stats and active pools. Phone credit → chain. Local balance tokenization is the primitive for SMS DeFi.</p>
      </div>
      <h2 class="dash-cards-title">Tokenized assets</h2>
      <div class="dash-asset-cards">
        ${TOKENIZED_ASSETS.map(
          (a) => {
            const half = halfTvl(a.tvl);
            return `
        <article class="dash-asset-card dash-asset-card--${a.active ? "active" : "inactive"}">
          <span class="dash-asset-card__status">${a.active ? "ACTIVE" : "COMING SOON"}</span>
          <h3 class="dash-asset-card__currency">${a.currency} (${a.code})</h3>
          <p class="dash-asset-card__country">${a.country}</p>
          ${half != null ? `<div class="dash-asset-card__tvl">${half}</div>` : ""}
        </article>`;
          }
        ).join("")}
      </div>
    </div>
  `;
}

/** Yield tab: yield opportunities as cards only */
export function renderYieldContent() {
  return `
    <div class="dash-panel dash-panel--yield">
      <h2 class="dash-cards-title">Yield opportunities</h2>
      <div class="dash-yield-cards">
        ${YIELD_OPPORTUNITIES.map(
          (y) => `
        <article class="dash-yield-card dash-yield-card--${y.active ? "active" : "inactive"}">
          <h3 class="dash-yield-card__pool">${y.pool}</h3>
          <p class="dash-yield-card__currency">${y.currency}</p>
          <div class="dash-yield-card__stats">
            <span class="dash-yield-card__stat"><em>APY</em> ${y.apy}</span>
            <span class="dash-yield-card__stat"><em>TVL</em> ${y.tvl}</span>
          </div>
          ${y.active ? '<span class="dash-yield-card__live">Live</span>' : '<span class="dash-yield-card__soon">Coming soon</span>'}
        </article>`
        ).join("")}
      </div>
    </div>
  `;
}
