# Deployment and Verification

## Pre-Deployment Checklist

Before deploying your migrated Stylus contract, ensure you've completed these essential steps:

**Contract Readiness Assessment:**
- [ ] All unit tests pass with `cargo test`
- [ ] Integration tests validate cross-contract interactions
- [ ] Gas usage is within acceptable limits (use testnet receipts, Foundry `cast estimate`, or your CI's simulation step)
- [ ] Error handling covers all edge cases
- [ ] Constructor parameters are properly validated
- [ ] Storage layout is optimized and finalized

**Security Considerations:**
- [ ] Access controls are properly implemented
- [ ] External calls use proper error handling
- [ ] No hardcoded addresses or sensitive data
- [ ] Reentrancy guards where necessary
- [ ] Integer overflow protection in place
- [ ] All public functions have appropriate visibility

**Gas Optimization Review:**
- [ ] Storage slots are packed efficiently
- [ ] Batch operations implemented where beneficial
- [ ] Unnecessary external calls eliminated
- [ ] Efficient algorithms used for computations
- [ ] Events emit only necessary data

## Local Deployment

### Setting up Stylus Test Node

For local development and testing, you'll need a local Arbitrum node with Stylus support:

```bash
# Clone and run the Nitro dev node with Stylus support
git clone https://github.com/OffchainLabs/nitro-devnode.git
cd nitro-devnode
./run-dev-node.sh

# The node will be available at http://localhost:8547
# The dev node prints funded accounts/keys on startup
# Use those ephemeral keys and never commit them to source control
```

**Environment Configuration:**
```toml
# In your Cargo.toml, add development dependencies
[dev-dependencies]
tokio = { version = "1.12.0", features = ["full"] }
ethers = "2.0"
eyre = "0.6.8"

[features]
export-abi = ["stylus-sdk/export-abi"]
```

Create a `.env` file for local testing:
```bash
# .env file for local development
# IMPORTANT: Never commit private keys or .env files; use .gitignore
RPC_URL=http://localhost:8547
# Use the funded key that the dev node printed at startup
PRIVATE_KEY=<paste-one-from-devnode-output>
```

### Constructor Handling

Stylus constructors work differently from Solana's initialization patterns. Here's how to properly implement them:

```rust
use stylus_sdk::prelude::*;
use stylus_sdk::alloy_primitives::{Address, U256};
use alloy_sol_types::sol;

sol! {
    error AlreadyInitialized();
    error InvalidOwner();
}

#[storage]
#[entrypoint]
pub struct MyContract {
    owner: StorageAddress,
    initial_value: StorageU256,
    initialized: StorageBool,
}

#[derive(SolidityError)]
pub enum ContractError {
    AlreadyInitialized(AlreadyInitialized),
    InvalidOwner(InvalidOwner),
}

#[public]
impl MyContract {
    #[constructor]
    pub fn constructor(&mut self, initial_value: U256, owner: Address) {
        // Use tx_origin instead of msg_sender because deployment uses a factory contract
        let deployer = self.vm().tx_origin();
        
        assert!(owner != Address::ZERO, "owner cannot be zero");
        
        self.owner.set(owner);
        self.initial_value.set(initial_value);
        self.initialized.set(true);
    }
    
    // Optional initializer for legacy patterns
    pub fn initialize(&mut self, initial_value: U256) -> Result<(), ContractError> {
        if self.initialized.get() {
            return Err(ContractError::AlreadyInitialized(AlreadyInitialized{}));
        }
        
        self.owner.set(msg::sender());
        self.initial_value.set(initial_value);
        self.initialized.set(true);
        Ok(())
    }
}
```

**Migration Pattern from Solana:**
```rust
// Solana Anchor initialization
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 32)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Stylus constructor pattern
#[constructor]
pub fn constructor(&mut self, initial_params: InitialParams) {
    // Initialization happens atomically during deployment
    // No separate initialization transaction needed
    self.state.set(initial_params.initial_state);
    self.owner.set(self.vm().tx_origin());
}
```

### Pre-deployment Setup for Constructor Support

If your chain doesn't have the required contracts for constructor support:

```bash
# Deploy CREATE2 factory (if not already deployed)
cast send --rpc-url $RPC --private-key $PRIVATE_KEY --value "1 ether" \
     0x3fab184622dc19b6109349b94811493bf2a45362

# Deploy StylusDeployer (required for constructors)
# Get the bytecode from the official repository
curl -O https://raw.githubusercontent.com/OffchainLabs/stylus-deployer/main/bytecode.txt
DEPLOYER_CODE=$(cat bytecode.txt)

# Deploy via CREATE2
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
cast send --private-key $PRIVATE_KEY --rpc-url $RPC \
    0x4e59b44847b379578588920ca78fbf26c0b4956c \
    "0x$SALT$DEPLOYER_CODE"
```

## Network Deployment

### Testnet Deployment

Deploy to Arbitrum Sepolia for testing:

```bash
# First, ensure you have test ETH
# Get testnet ETH from: https://www.alchemy.com/faucets/arbitrum-sepolia

# Check your contract compiles for Stylus
cargo stylus check --endpoint https://sepolia-rollup.arbitrum.io/rpc

# Deploy to testnet with constructor args
cargo stylus deploy \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --private-key-path ~/.secrets/testnet.key \
    --constructor-args 1000000 0x742d35Cc6634C0532925a3b8D0bCe67D23F15f3C
```

**Constructor Arguments File (constructor-args.json):**
```json
{
  "args": ["1000000", "0x742d35Cc6634C0532925a3b8D0bCe67D23F15f3C"]
}
```

**Environment Setup for Testnet:**
```bash
# .env.testnet file
ARBITRUM_SEPOLIA_RPC=https://sepolia-rollup.arbitrum.io/rpc
TESTNET_PRIVATE_KEY_PATH=~/.secrets/testnet.key
ARBISCAN_API_KEY=your_arbiscan_api_key

# Export for use
source .env.testnet
```

### Mainnet Deployment

**Pre-flight Checks:**
```bash
# Comprehensive pre-deployment validation
echo "Running pre-deployment checks..."

# 1. Check contract compiles and is valid
cargo stylus check --endpoint https://arb1.arbitrum.io/rpc

# 2. Run all tests
cargo test --release -- --nocapture

# 3. Check ETH balance
DEPLOYER_ADDRESS=$(cast wallet address --private-key-path ~/.secrets/mainnet.key)
BALANCE=$(cast balance $DEPLOYER_ADDRESS --rpc-url https://arb1.arbitrum.io/rpc)
echo "Deployer balance: $BALANCE"

# 4. Verify sufficient balance (deployment typically costs 0.01-0.1 ETH)
# Manual check required here
```

**Deployment Process:**
```bash
# Deploy to Arbitrum One mainnet
cargo stylus deploy \
    --endpoint https://arb1.arbitrum.io/rpc \
    --private-key-path ~/.secrets/mainnet.key \
    --constructor-args 1000000 0x742d35Cc6634C0532925a3b8D0bCe67D23F15f3C \
    --gas-price 100000000 \
    --no-verify

# Save deployment address
DEPLOYED_ADDRESS=$(cargo stylus deploy ... | grep "Deployed to" | awk '{print $3}')
echo "Contract deployed to: $DEPLOYED_ADDRESS"
```

## Using cargo stylus deploy

### Basic Deployment

The `cargo stylus deploy` command handles contract compilation, optimization, and deployment:

```bash
# Basic deployment command structure
cargo stylus deploy \
    --endpoint <RPC_ENDPOINT> \
    --private-key-path <PRIVATE_KEY_PATH> \
    [--constructor-args <ARGS>...] \
    [--no-verify]

# Example with all common options
cargo stylus deploy \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --private-key-path ~/.secrets/testnet.key \
    --constructor-args 1000 0xABC123... \
    --gas-limit 30000000 \
    --gas-price 100000000 \
    --no-verify
```

**Configuration Options:**

| Option | Description | Example |
|--------|-------------|---------|
| `--endpoint` | RPC endpoint URL | `https://arb1.arbitrum.io/rpc` |
| `--private-key-path` | Path to private key file | `~/.secrets/key` |
| `--private-key` | Private key directly (not recommended) | `0x123...` |
| `--constructor-args` | Constructor parameters | `arg1 arg2` |
| `--constructor-args-path` | Path to JSON file with args | `./args.json` |
| `--gas-limit` | Maximum gas for deployment | `30000000` |
| `--gas-price` | Gas price in wei | `100000000` |
| `--no-verify` | Skip verification step | No value needed |

### Advanced Deployment Options

**Using Configuration Files:**
```toml
# deployment.toml
[deployment]
endpoint = "https://sepolia-rollup.arbitrum.io/rpc"
private_key_path = "~/.secrets/testnet.key"
gas_limit = 30000000
gas_price = 100000000

[constructor]
args = ["1000", "0x742d35Cc6634C0532925a3b8D0bCe67D23F15f3C"]

[verification]
arbiscan_api_key = "${ARBISCAN_API_KEY}"
```

```bash
# Deploy using configuration
cargo stylus deploy --config deployment.toml
```

**Complex Constructor Arguments:**
```rust
// Contract with complex constructor
#[constructor]
pub fn constructor(
    &mut self,
    config: DeploymentConfig,
    initial_operators: Vec<Address>,
    fee_schedule: FeeSchedule,
) {
    // Handle complex initialization
    self.config.set(config);
    for (i, op) in initial_operators.iter().enumerate() {
        self.operators.setter(U256::from(i)).set(*op);
    }
    self.fees.set(fee_schedule);
}
```

```bash
# Deploy with complex arguments using JSON file
cat > constructor-args.json << EOF
{
  "args": [
    {
      "name": "MyToken",
      "symbol": "MTK",
      "decimals": 18
    },
    ["0x123...", "0x456...", "0x789..."],
    {
      "transfer_fee": 100,
      "mint_fee": 50,
      "burn_fee": 25
    }
  ]
}
EOF

cargo stylus deploy \
    --endpoint $RPC_URL \
    --private-key-path $KEY_PATH \
    --constructor-args-path constructor-args.json
```

### Deployment Artifacts

After deployment, save important artifacts:

```bash
# Create deployment record
mkdir -p deployments/$(date +%Y%m%d)
cd deployments/$(date +%Y%m%d)

# Save deployment info
cat > deployment-info.json << EOF
{
  "network": "arbitrum-sepolia",
  "contract_address": "$DEPLOYED_ADDRESS",
  "deployer": "$DEPLOYER_ADDRESS",
  "deployment_tx": "$DEPLOYMENT_TX",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "constructor_args": $(cat ../../constructor-args.json),
  "gas_used": "$GAS_USED",
  "contract_size": "$(stat -f%z ../../target/wasm32-unknown-unknown/release/contract.wasm)"
}
EOF

# Export ABI
cargo stylus export-abi > contract-abi.json

# Save source code snapshot
git archive HEAD --format=tar.gz > source-snapshot.tar.gz
```

## Contract Verification

### Using cargo stylus verify

Contract verification makes your source code publicly available and auditable:

```bash
# Basic verification (immediately after deployment)
cargo stylus verify \
    --deployment-tx $DEPLOYMENT_TX \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc

# Verification with specific contract address
cargo stylus verify \
    --contract-address $DEPLOYED_ADDRESS \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --arbiscan-api-key $ARBISCAN_API_KEY

# Verification with constructor arguments
cargo stylus verify \
    --contract-address $DEPLOYED_ADDRESS \
    --constructor-args-path ./constructor-args.json \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --arbiscan-api-key $ARBISCAN_API_KEY
```

**Verification Requirements:**
- Exact same source code as deployed
- Same Rust toolchain version
- Same cargo-stylus version
- Correct constructor arguments (if any)
- Valid Arbiscan API key

**Troubleshooting Verification:**
```bash
# Check contract bytecode matches
cargo stylus check \
    --contract-address $DEPLOYED_ADDRESS \
    --endpoint $RPC_URL

# Verify toolchain versions match
rustc --version > deployment-toolchain.txt
cargo stylus --version >> deployment-toolchain.txt

# Debug verification with verbose output
cargo stylus verify \
    --contract-address $DEPLOYED_ADDRESS \
    --endpoint $RPC_URL \
    --arbiscan-api-key $ARBISCAN_API_KEY \
    --verbose

# Manual verification fallback
# If automated verification fails, use Arbiscan UI:
# 1. Go to https://arbiscan.io/verifyContract
# 2. Select "Stylus Contract" as compiler type
# 3. Upload your source code and constructor args
```

### Block Explorer Integration

Once verified, your contract gains full functionality on Arbiscan:

**Benefits of Verification:**
- Source code is publicly viewable
- ABI is automatically extracted and displayed
- Users can interact directly through Arbiscan UI
- Transaction decoding shows human-readable function calls
- Event logs are automatically decoded

**Post-Verification Features:**
```rust
// Your verified contract on Arbiscan will show:
// 1. Read Contract tab - for view functions
pub fn balance_of(&self, account: Address) -> U256 {
    self.balances.get(account)
}

// 2. Write Contract tab - for state-changing functions
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Error> {
    // Users can call this directly from Arbiscan
    self._transfer(msg::sender(), to, amount)?;
    Ok(true)
}

// 3. Events tab - decoded event logs
evm::log(Transfer {
    from: msg::sender(),
    to,
    value: amount,
});
```

**Explorer URLs:**
- **Mainnet**: `https://arbiscan.io/address/{CONTRACT_ADDRESS}`
- **Sepolia**: `https://sepolia.arbiscan.io/address/{CONTRACT_ADDRESS}`

## Post-Deployment

### Monitoring and Maintenance

Set up comprehensive monitoring for your deployed contract:

```rust
// Add detailed events for monitoring
use stylus_sdk::prelude::*;
use alloy_sol_types::sol;

sol! {
    // Operational events
    event StateChanged(address indexed user, uint256 oldValue, uint256 newValue, uint256 timestamp);
    event FunctionCalled(address indexed caller, bytes4 indexed selector, uint256 gasUsed);
    
    // Error events for monitoring
    event ErrorOccurred(address indexed user, string reason, bytes data);
    event AccessDenied(address indexed user, bytes4 indexed function);
    
    // Admin events
    event AdminAction(address indexed admin, string action, bytes params);
    event EmergencyStop(address indexed admin, string reason);
}

#[public]
impl MyContract {
    pub fn update_state(&mut self, new_value: U256) -> Result<(), Error> {
        let old_value = self.state.get();
        let caller = msg::sender();
        
        // Access control with event
        if !self.authorized.get(caller) {
            evm::log(AccessDenied {
                user: caller,
                function: function_selector!("update_state", U256),
            });
            return Err(Error::Unauthorized);
        }
        
        self.state.set(new_value);
        
        // Detailed event for monitoring
        evm::log(StateChanged {
            user: caller,
            old_value,
            new_value,
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(())
    }
}
```

**Monitoring Setup Script:**
```javascript
// monitoring.js - Event monitoring service
const { ethers } = require("ethers");
const fs = require("fs");

// Load contract ABI and address
const abi = JSON.parse(fs.readFileSync("./contract-abi.json"));
const contractAddress = process.env.CONTRACT_ADDRESS;

// Setup provider and contract
const provider = new ethers.JsonRpcProvider(process.env.RPC_URL);
const contract = new ethers.Contract(contractAddress, abi, provider);

// State change monitoring
contract.on("StateChanged", async (user, oldValue, newValue, timestamp, event) => {
    console.log(`State change detected:`);
    console.log(`  User: ${user}`);
    console.log(`  Change: ${oldValue} -> ${newValue}`);
    console.log(`  Block: ${event.blockNumber}`);
    console.log(`  Gas used: ${event.gasUsed}`);
    
    // Send to monitoring system (e.g., Grafana, Datadog)
    await sendMetric("state_change", {
        user,
        old_value: oldValue.toString(),
        new_value: newValue.toString(),
        block: event.blockNumber,
    });
});

// Error monitoring
contract.on("ErrorOccurred", async (user, reason, data, event) => {
    console.error(`Error detected for user ${user}: ${reason}`);
    
    // Alert administrators
    await sendAlert({
        severity: "high",
        user,
        reason,
        data: ethers.hexlify(data),
        tx_hash: event.transactionHash,
    });
});

// Performance monitoring
contract.on("FunctionCalled", async (caller, selector, gasUsed, event) => {
    // Track gas usage patterns
    await sendMetric("gas_usage", {
        function: selector,
        gas: gasUsed.toString(),
        caller,
    });
});
```

### Performance Tracking

Monitor and optimize gas usage patterns:

```bash
#!/bin/bash
# gas-monitor.sh - Track gas usage over time

CONTRACT=$1
RPC_URL=$2

# Get recent transactions
RECENT_TXS=$(cast logs \
    --address $CONTRACT \
    --from-block -1000 \
    --rpc-url $RPC_URL \
    | jq -r '.[].transactionHash' \
    | sort -u)

# Analyze gas usage
echo "Function,GasUsed,GasPrice,TotalCost"
for TX in $RECENT_TXS; do
    TX_DATA=$(cast tx $TX --rpc-url $RPC_URL --json)
    FUNCTION=$(echo $TX_DATA | jq -r '.input[0:10]')
    GAS_USED=$(echo $TX_DATA | jq -r '.gasUsed')
    GAS_PRICE=$(echo $TX_DATA | jq -r '.gasPrice')
    COST=$(echo "scale=6; $GAS_USED * $GAS_PRICE / 1000000000000000000" | bc)
    
    echo "$FUNCTION,$GAS_USED,$GAS_PRICE,$COST"
done | sort | uniq -c | sort -rn
```

### Upgrade Considerations

Plan for future upgrades using proxy patterns:

```rust
// Minimal proxy implementation for upgrades
use stylus_sdk::prelude::*;
use stylus_sdk::call::{RawCall, delegate_call};

#[storage]
#[entrypoint]
pub struct UpgradeableProxy {
    implementation: StorageAddress,
    admin: StorageAddress,
    version: StorageU256,
}

sol! {
    event Upgraded(address indexed oldImplementation, address indexed newImplementation, uint256 version);
    error NotAdmin();
    error InvalidImplementation();
}

#[public]
impl UpgradeableProxy {
    #[payable]
    #[fallback]
    pub fn fallback(&mut self, input: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        let implementation = self.implementation.get();
        
        // Delegate all calls to implementation
        unsafe {
            delegate_call(self, implementation, input)
                .map_err(|_| b"Delegate call failed".to_vec())
        }
    }
    
    pub fn upgrade(&mut self, new_implementation: Address) -> Result<(), Error> {
        if msg::sender() != self.admin.get() {
            return Err(Error::NotAdmin(NotAdmin{}));
        }
        
        // Validate new implementation
        if new_implementation == Address::ZERO {
            return Err(Error::InvalidImplementation(InvalidImplementation{}));
        }
        
        let old_implementation = self.implementation.get();
        self.implementation.set(new_implementation);
        self.version.set(self.version.get() + U256::from(1));
        
        evm::log(Upgraded {
            old_implementation,
            new_implementation,
            version: self.version.get(),
        });
        
        Ok(())
    }
}
```

## Documentation

### Generating User Documentation

Create comprehensive documentation for contract users:

```bash
#!/bin/bash
# generate-docs.sh - Generate contract documentation

CONTRACT_ADDRESS=$1
NETWORK=$2
DEPLOYMENT_TX=$3

# Generate ABI documentation
cargo stylus export-abi > docs/abi.json

# Create deployment summary
cat > docs/deployment-summary.md << EOF
# Contract Deployment Summary

## Network Information
- **Network**: $NETWORK
- **Contract Address**: \`$CONTRACT_ADDRESS\`
- **Deployment Transaction**: [$DEPLOYMENT_TX](https://arbiscan.io/tx/$DEPLOYMENT_TX)
- **Deployed On**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
- **Compiler Version**: $(cargo stylus --version)
- **Rust Version**: $(rustc --version)

## Verification Status
- **Verified**: Yes
- **Arbiscan**: [View Contract](https://arbiscan.io/address/$CONTRACT_ADDRESS#code)

## Constructor Parameters
$(cat constructor-args.json | jq -r '.args | @json')

## Gas Statistics
- **Deployment Gas Used**: $(cast tx $DEPLOYMENT_TX --rpc-url $RPC_URL | jq -r '.gasUsed')
- **Contract Size**: $(stat -f%z target/wasm32-unknown-unknown/release/*.wasm) bytes

## Key Functions

### Read Functions
$(cat docs/abi.json | jq -r '.[] | select(.stateMutability == "view") | "- \`" + .name + "\`"')

### Write Functions
$(cat docs/abi.json | jq -r '.[] | select(.stateMutability != "view") | "- \`" + .name + "\`"')

## Integration Guide
See [integration.md](./integration.md) for detailed integration instructions.
EOF
```

**Integration Guide Template:**
```markdown
# Integration Guide

## Quick Start

### JavaScript/TypeScript
\`\`\`javascript
import { ethers } from 'ethers';
import contractABI from './abi.json';

const CONTRACT_ADDRESS = '0x...';

// Connect to contract
const provider = new ethers.JsonRpcProvider('https://arb1.arbitrum.io/rpc');
const contract = new ethers.Contract(CONTRACT_ADDRESS, contractABI, provider);

// Read operations
const balance = await contract.balanceOf(userAddress);
const totalSupply = await contract.totalSupply();

// Write operations (requires signer)
const signer = await provider.getSigner();
const contractWithSigner = contract.connect(signer);
const tx = await contractWithSigner.transfer(recipientAddress, amount);
await tx.wait();
\`\`\`

### Rust Client
\`\`\`rust
use ethers::prelude::*;

// Contract ABI and address
abigen!(
    MyContract,
    "./abi.json"
);

let contract = MyContract::new(contract_address, client);
let balance = contract.balance_of(user_address).call().await?;
\`\`\`

## Gas Costs

| Function | Typical Gas | Max Gas | Cost (at 0.1 gwei) |
|----------|------------|---------|-------------------|
| transfer | 65,000 | 80,000 | ~$0.001 |
| approve | 45,000 | 55,000 | ~$0.0007 |
| mint | 85,000 | 100,000 | ~$0.0015 |
```

### Migration Guide for Users

Help users transition from your Solana dApp:

```markdown
# Migration Guide: Solana to Arbitrum

## For End Users

### 1. Wallet Setup
- **From**: Phantom, Solflare, or other Solana wallets
- **To**: MetaMask, Rabby, or other Ethereum wallets
- **Network**: Add Arbitrum One network:
  - RPC URL: https://arb1.arbitrum.io/rpc
  - Chain ID: 42161
  - Currency: ETH

### 2. Address Format
- **Solana**: Base58 format (e.g., \`7EYnhQoR9YM...\`)
- **Arbitrum**: Hex format (e.g., \`0x742d35Cc...\`)
- Your new address will be different - you cannot convert between them

### 3. Getting ETH for Gas
- Bridge ETH from Ethereum: https://bridge.arbitrum.io
- Buy directly on Arbitrum via exchanges
- Gas costs are typically $0.001-0.01 per transaction

### 4. Transaction Differences
| Aspect | Solana | Arbitrum |
|--------|---------|----------|
| Speed | ~400ms | 1-2 seconds |
| Fees | ~$0.00025 | ~$0.001-0.01 |
| Confirmations | Instant finality | Wait for 1-2 blocks |
| Failed txs | No fee | Fee still charged |

## For Developers

### API Migration
\`\`\`javascript
// Solana (using @solana/web3.js)
const connection = new Connection(clusterApiUrl('mainnet-beta'));
const publicKey = new PublicKey('...');
const balance = await connection.getBalance(publicKey);

// Arbitrum (using ethers.js)
const provider = new ethers.JsonRpcProvider('https://arb1.arbitrum.io/rpc');
const address = '0x...';
const balance = await provider.getBalance(address);
\`\`\`

### Key Differences
1. **No Program Accounts**: State is stored in contract storage
2. **No Instructions**: Direct function calls instead
3. **Different SDKs**: Ethers.js/Viem instead of @solana/web3.js
4. **Gas Model**: Pay per computation, not fixed compute units
```

## Next Steps

With your contract successfully deployed and verified, you're ready to explore advanced Stylus features. The next chapter covers:

- **Inheritance patterns** for code reuse
- **Zero-copy optimizations** for large data structures
- **Advanced call patterns** including delegate calls and multi-calls
- **Assembly optimizations** for critical paths

Remember to:
1. **Monitor your deployed contracts** using the scripts provided
2. **Maintain documentation** as you add features
3. **Track gas usage** to identify optimization opportunities
4. **Engage with users** during the migration process
5. **Keep security audits updated** as you modify the contract

The successful deployment of your Stylus contract marks a significant milestone in your migration journey from Solana to Arbitrum. The patterns and tools covered in this chapter will serve you well as you continue to build and optimize your decentralized applications.