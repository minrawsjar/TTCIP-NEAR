/**
 * Register an Entity Secret with Circle.
 * 
 * Usage:
 *   1. Set CIRCLE_API_KEY and CIRCLE_ENTITY_SECRET in .env
 *   2. Run: npx ts-node setup-entity-secret.ts
 */

import { registerEntitySecretCiphertext } from '@circle-fin/developer-controlled-wallets';
import dotenv from 'dotenv';
import path from 'path';
import fs from 'fs';

dotenv.config();

async function main() {
  const apiKey = process.env.CIRCLE_API_KEY;
  const entitySecret = process.env.CIRCLE_ENTITY_SECRET;

  if (!apiKey) {
    console.error('❌ Set CIRCLE_API_KEY in .env first!');
    process.exit(1);
  }
  if (!entitySecret) {
    console.error('❌ Set CIRCLE_ENTITY_SECRET in .env first!');
    process.exit(1);
  }

  console.log('� Registering Entity Secret with Circle...\n');
  console.log(`API Key: ${apiKey.substring(0, 20)}...`);
  console.log(`Entity Secret: ${entitySecret.substring(0, 12)}...`);

  try {
    // SDK expects a directory path, not a file path
    const recoveryPath = process.cwd();
    
    const response = await registerEntitySecretCiphertext({
      apiKey: apiKey,
      entitySecret: entitySecret,
      recoveryFileDownloadPath: recoveryPath,
    });

    console.log('\n✅ Entity Secret registered successfully!');
    if (fs.existsSync(recoveryPath)) {
      console.log(`Recovery file saved to: ${recoveryPath}`);
      console.log('⚠️  Keep this file safe — it\'s the only way to reset your Entity Secret!');
    }
  } catch (error: any) {
    console.error('\n❌ Registration failed:', error.message || error);
  }
}

main();
