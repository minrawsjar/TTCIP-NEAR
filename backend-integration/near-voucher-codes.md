# NEAR Voucher Codes

## Active Vouchers (500 TXTC + 0.1 NEAR gas each)

### Welcome Codes (High Value)
1. **WELCOME500** - 500 TXTC + 0.1 NEAR gas
2. **NEARSTART** - 500 TXTC + 0.1 NEAR gas
3. **TXTC2024** - 500 TXTC + 0.1 NEAR gas

### Standard Codes (100 TXTC + 0.05 NEAR gas each)
4. **NEAR100A** - 100 TXTC + 0.05 NEAR
5. **NEAR100B** - 100 TXTC + 0.05 NEAR
6. **NEAR100C** - 100 TXTC + 0.05 NEAR
7. **NEAR100D** - 100 TXTC + 0.05 NEAR
8. **NEAR100E** - 100 TXTC + 0.05 NEAR

### Testing Codes (10 TXTC + 0.01 NEAR gas each)
9. **TEST001** - 10 TXTC + 0.01 NEAR
10. **TEST002** - 10 TXTC + 0.01 NEAR
11. **TEST003** - 10 TXTC + 0.01 NEAR
12. **TEST004** - 10 TXTC + 0.01 NEAR
13. **TEST005** - 10 TXTC + 0.01 NEAR

## Usage

### Via SMS:
```
REDEEM WELCOME500
```

### Via API:
```bash
curl -X POST http://localhost:3000/api/redeem \
  -H "Content-Type: application/json" \
  -d '{
    "voucherCode": "WELCOME500",
    "userAddress": "alice.0xswarnim.testnet"
  }'
```

### Via Local SMS Test:
```bash
curl -X POST http://localhost:8080/sms/incoming \
  -d "From=+15551234567&Body=REDEEM WELCOME500"
```

## Voucher Details

- **TXTC Token**: `txtc.0xswarnim.testnet`
- **Pool Contract**: `ttcip-pool.0xswarnim.testnet`
- **Network**: NEAR Testnet
- **Redemption**: One-time use per code
- **Gas Bonus**: Includes NEAR for transaction fees

## Notes

1. Voucher codes are **case-insensitive**
2. Each code can only be redeemed **once**
3. Gas bonuses help new users pay for their first transactions
4. Codes must be redeemed to a valid NEAR account (create with `JOIN <name>` first)
