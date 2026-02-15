/**
 * Pools panel and mock data for TTC-IP dApp
 * Nigerian pool active and first; rest inactive.
 */
export const mockPools = [
  { country: "NG", countryName: "Nigeria", provider: "MTN", tvl: "1.2M", apyTtc: "25%", active: true, currency: "Naira", currencyCode: "NGN" },
  { country: "IN", countryName: "India", provider: "Airtel", tvl: "—", apyTtc: "—", active: false, currency: "Rupee", currencyCode: "INR" },
  { country: "KE", countryName: "Kenya", provider: "Safaricom", tvl: "—", apyTtc: "—", active: false, currency: "Shilling", currencyCode: "KES" },
  { country: "BR", countryName: "Brazil", provider: "Claro", tvl: "—", apyTtc: "—", active: false, currency: "Real", currencyCode: "BRL" },
  { country: "SN", countryName: "Senegal", provider: "Orange", tvl: "—", apyTtc: "—", active: false, currency: "CFA Franc", currencyCode: "XOF" },
  { country: "GH", countryName: "Ghana", provider: "MTN", tvl: "—", apyTtc: "—", active: false, currency: "Cedi", currencyCode: "GHS" },
  { country: "TZ", countryName: "Tanzania", provider: "Vodacom", tvl: "—", apyTtc: "—", active: false, currency: "Shilling", currencyCode: "TZS" },
  { country: "ZA", countryName: "South Africa", provider: "Vodacom", tvl: "—", apyTtc: "—", active: false, currency: "Rand", currencyCode: "ZAR" },
  { country: "IN", countryName: "India", provider: "Jio", tvl: "—", apyTtc: "—", active: false, currency: "Rupee", currencyCode: "INR" },
];

function poolDescription(p) {
  return `${p.currency} (${p.currencyCode}) — ${p.provider} value unit`;
}

export function renderPoolsPanel(filters = {}) {
  const { country = "", currency = "", provider: providerFilter = "" } = filters;
  const filtered = mockPools.filter((p) => {
    if (country && !p.country.toLowerCase().includes(country.toLowerCase())) return false;
    if (providerFilter && !p.provider.toLowerCase().includes(providerFilter.toLowerCase())) return false;
    return true;
  });
  const providers = [...new Set(mockPools.map((p) => p.provider))].sort();
  const countries = [...new Set(mockPools.map((p) => p.country))].sort();
  return `
    <div class="pools-panel__top">
      <h1 class="pools-panel__title">Pools</h1>
      <div class="dash-filters dash-filters--row">
        <select class="dash-filter__select" data-filter="country" aria-label="Filter by country">
          <option value="">All countries</option>
          ${countries.map((c) => `<option value="${c}" ${country === c ? "selected" : ""}>${c}</option>`).join("")}
        </select>
        <select class="dash-filter__select" data-filter="provider" aria-label="Filter by provider">
          <option value="">All providers</option>
          ${providers.map((pr) => `<option value="${pr}" ${providerFilter === pr ? "selected" : ""}>${pr}</option>`).join("")}
        </select>
        <select class="dash-filter__select" data-filter="currency" aria-label="Filter by currency">
          <option value="">All</option>
          <option value="USDC" ${currency === "USDC" ? "selected" : ""}>USDC</option>
        </select>
      </div>
    </div>
    <div class="pools-grid">
      ${filtered.map((p) => `
        <article class="pool-card pool-card--${p.active ? "active" : "inactive"}${p.active ? " pool-card--clickable" : ""}" ${p.active ? `data-pool-id="${p.country}-${p.provider}"` : ""} role="${p.active ? "button" : ""}" tabindex="${p.active ? "0" : "-1"}">
          <div class="pool-card__header">
            <h3 class="pool-card__country">${p.currency} (${p.currencyCode})</h3>
            <span class="pool-card__status pool-card__status--${p.active ? "live" : "soon"}">${p.active ? "ACTIVE" : "COMING SOON"}</span>
          </div>
          <p class="pool-card__provider">${p.countryName} · ${p.provider}</p>
          <p class="pool-card__desc">${poolDescription(p)}</p>
          <div class="pool-card__stats">
            <div class="pool-card__stat">
              <span class="pool-card__stat-label">TVL</span>
              <span class="pool-card__stat-value">${p.tvl}</span>
            </div>
            <div class="pool-card__stat">
              <span class="pool-card__stat-label">APY (TTC)</span>
              <span class="pool-card__stat-value pool-card__stat-value--yield">${p.apyTtc || "—"}</span>
            </div>
          </div>
        </article>
      `).join("")}
    </div>
  `;
}
