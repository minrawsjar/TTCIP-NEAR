/**
 * NEAR Name Service - ENS-like functionality for NEAR subdomains
 * Manages *.0xswarnim.testnet subdomain registrations
 */

export class NearNameService {
    private ownerAccount: string;
    private registeredNames: Map<string, string> = new Map();

    constructor() {
        this.ownerAccount = process.env.NEAR_OWNER_ACCOUNT || '0xswarnim.testnet';
        console.log(`🏷️  NEAR Name Service initialized for ${this.ownerAccount}`);
    }

    /**
     * Check if a name is available for registration
     */
    async checkAvailability(name: string): Promise<{
        available: boolean;
        reason?: string;
    }> {
        const cleanName = name.toLowerCase().trim();

        // Validate name format
        if (!/^[a-z0-9_-]+$/.test(cleanName)) {
            return {
                available: false,
                reason: 'Name can only contain letters, numbers, hyphens, and underscores',
            };
        }

        if (cleanName.length < 2) {
            return {
                available: false,
                reason: 'Name must be at least 2 characters',
            };
        }

        if (cleanName.length > 64) {
            return {
                available: false,
                reason: 'Name must be less than 64 characters',
            };
        }

        // Check if already registered
        if (this.registeredNames.has(cleanName)) {
            return {
                available: false,
                reason: 'Name is already registered',
            };
        }

        return { available: true };
    }

    /**
     * Register a subdomain
     */
    async registerSubdomain(
        name: string,
        walletAddress: string
    ): Promise<{
        success: boolean;
        ensName?: string;
        nearAccount?: string;
        txHash?: string;
        error?: string;
    }> {
        const check = await this.checkAvailability(name);
        if (!check.available) {
            return {
                success: false,
                error: check.reason,
            };
        }

        const cleanName = name.toLowerCase().trim();
        const subdomain = `${cleanName}.${this.ownerAccount}`;

        // Store mapping
        this.registeredNames.set(cleanName, walletAddress);

        console.log(`✅ Registered ${subdomain} → ${walletAddress}`);

        return {
            success: true,
            ensName: subdomain,
            nearAccount: subdomain,
            txHash: 'mock-tx-hash-' + Date.now(),
        };
    }

    /**
     * Resolve a name to an address
     */
    async resolveAddress(name: string): Promise<string | null> {
        const cleanName = name.toLowerCase().trim().replace(`.${this.ownerAccount}`, '');
        return this.registeredNames.get(cleanName) || null;
    }
}

// Export singleton instance
export const nearNameService = new NearNameService();
