/**
 * NEAR Smart Contract Configuration for Text-to-Chain Backend
 * NEAR Testnet Deployment
 */

export const NEAR_TESTNET_CONFIG = {
  networkId: 'testnet',
  nodeUrl: process.env.NEAR_RPC_URL || 'https://rpc.testnet.near.org',
  walletUrl: 'https://testnet.mynearwallet.com/',
  helperUrl: 'https://helper.testnet.near.org',
  explorerUrl: 'https://explorer.testnet.near.org',
  
  contracts: {
    poolContract: process.env.NEAR_POOL_CONTRACT || 'ttcip-pool.0xswarn.testnet',
  },
  
  // NEAR native token (no need for separate token contracts)
  nativeToken: {
    symbol: 'NEAR',
    decimals: 24,
  },
  
  // Pool configuration
  poolConfig: {
    minDeposit: '1000000000000000000000000', // 1 NEAR minimum deposit
    ownerAccount: process.env.NEAR_OWNER_ACCOUNT || '0xswarn.testnet',
  },
} as const;

export type NearContractAddresses = typeof NEAR_TESTNET_CONFIG.contracts;
