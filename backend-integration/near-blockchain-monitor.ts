/**
 * NEAR Blockchain Monitor - Stub Implementation
 * Monitors NEAR blockchain events (deposits, transfers, etc.)
 */

export class NearBlockchainMonitor {
    constructor() {
        console.log('📡 NEAR Blockchain Monitor initialized (stub)');
    }

    async start() {
        console.log('📡 NEAR Blockchain Monitor started');
    }

    async stop() {
        console.log('📡 NEAR Blockchain Monitor stopped');
    }
}

// Export singleton instance
export const nearBlockchainMonitor = new NearBlockchainMonitor();
