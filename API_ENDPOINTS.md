# Wallet API Endpoints Documentation

This document defines the API endpoints that the wallet frontend expects to be available. These endpoints should be implemented in the Tauri backend and/or blockchain node API.

## RPC Naming Standard

Chert now follows an Ethereum-style namespacing scheme for JSON-RPC methods: `domain_action`, all lowercase with underscores. This prevents collisions across domains while staying compatible with existing tooling. Legacy route names (without namespaces) are listed for reference; they should be considered deprecated and supported only for backwards compatibility.

## Staking APIs

### Get Validators
**Route:** `staking_get_validators`
**Legacy Route:** `get_validators`
**Method:** Tauri command
**Description:** Retrieve list of available validators for staking
**Request:**
```typescript
interface GetValidatorsRequest {
  // Optional filters
  status?: 'active' | 'inactive' | 'all';
  limit?: number;
  offset?: number;
}
```
**Response:**
```typescript
interface ValidatorInfo {
  address: string;
  publicKey: string;
  networkKey: string;
  stake: number;
  stakeAmount: number;
  isActive: boolean;
  commissionRate: number; // 0-100 percentage
  reputationScore: number; // 0-100 score
  lastActivity: string; // ISO timestamp
  totalDelegated?: number;
  delegatorCount?: number;
}
```

### Get User Delegations
**Route:** `staking_get_user_delegations`
**Legacy Route:** `get_user_delegations`
**Method:** Tauri command
**Description:** Get current user's staking delegations
**Request:**
```typescript
interface GetUserDelegationsRequest {
  userAddress: string;
}
```
**Response:**
```typescript
interface DelegationInfo {
  delegator: string;
  validator: string;
  amount: number; // in smallest unit
  shares: number;
  delegatedAt: string; // ISO timestamp
  rewardsClaimed: number;
}
```

### Get Staking Rewards
**Route:** `staking_get_rewards`
**Legacy Route:** `get_staking_rewards`
**Method:** Tauri command
**Description:** Get user's staking rewards information
**Request:**
```typescript
interface GetStakingRewardsRequest {
  userAddress: string;
}
```
**Response:**
```typescript
interface RewardInfo {
  totalEarned: number;
  pendingRewards: number;
  currentAPY: number; // percentage
}
```

### Get Lockbox Records
**Route:** `staking_get_lockbox_records`
**Legacy Route:** `get_lockbox_records`
**Method:** Tauri command
**Description:** Get user's lockbox staking records
**Request:**
```typescript
interface GetLockboxRecordsRequest {
  userAddress: string;
}
```
**Response:**
```typescript
interface LockBoxRecord {
  account: string;
  amount: number;
  termMonths: number;
  lockedAt: string; // ISO timestamp
  unlockAt: string; // ISO timestamp
  multiplier: number; // reward multiplier
  isActive: boolean;
}
```

### Get Auto-Stake Status
**Route:** `staking_get_auto_stake_status`
**Legacy Route:** `get_auto_stake_status`
**Method:** Tauri command
**Description:** Get user's auto-staking configuration
**Request:**
```typescript
interface GetAutoStakeStatusRequest {
  userAddress: string;
}
```
**Response:**
```typescript
interface AutoStakeRecord {
  account: string;
  balance: number;
  maturityTimestamp: string; // ISO timestamp
  isActive: boolean;
}
```

### Delegate Tokens
**Route:** `staking_delegate_tokens`
**Legacy Route:** `delegate_tokens`
**Method:** Tauri command
**Description:** Delegate tokens to a validator
**Request:**
```typescript
interface DelegateTokensRequest {
  delegatorAddress: string;
  validatorAddress: string;
  amount: number; // in smallest unit
}
```
**Response:**
```typescript
interface DelegateTokensResponse {
  transactionId: string;
  delegation: DelegationInfo;
}
```

### Undelegate Tokens
**Route:** `staking_undelegate_tokens`
**Legacy Route:** `undelegate_tokens`
**Method:** Tauri command
**Description:** Remove delegation from a validator
**Request:**
```typescript
interface UndelegateTokensRequest {
  delegatorAddress: string;
  validatorAddress: string;
  amount: number; // in smallest unit
}
```
**Response:**
```typescript
interface UndelegateTokensResponse {
  transactionId: string;
  updatedDelegation: DelegationInfo;
}
```

### Create Lockbox Stake
**Route:** `staking_create_lockbox_stake`
**Legacy Route:** `create_lockbox_stake`
**Method:** Tauri command
**Description:** Create a time-locked staking position
**Request:**
```typescript
interface CreateLockboxStakeRequest {
  account: string;
  amount: number; // in smallest unit
  termMonths: number;
}
```
**Response:**
```typescript
interface CreateLockboxStakeResponse {
  transactionId: string;
  lockboxRecord: LockBoxRecord;
}
```

### Toggle Auto-Staking
**Route:** `staking_toggle_auto_staking`
**Legacy Route:** `toggle_auto_staking`
**Method:** Tauri command
**Description:** Enable or disable auto-staking for rewards
**Request:**
```typescript
interface ToggleAutoStakingRequest {
  account: string;
  enable: boolean;
}
```
**Response:**
```typescript
interface ToggleAutoStakingResponse {
  success: boolean;
  autoStakeStatus: AutoStakeRecord;
}
```

### Claim Staking Rewards
**Route:** `staking_claim_rewards`
**Legacy Route:** `claim_staking_rewards`
**Method:** Tauri command
**Description:** Claim accumulated staking rewards
**Request:**
```typescript
interface ClaimStakingRewardsRequest {
  account: string;
}
```
**Response:**
```typescript
interface ClaimStakingRewardsResponse {
  transactionId: string;
  claimedAmount: number;
}
```

## Governance APIs

### Get Proposals
**Route:** `governance_list_proposals`
**Legacy Route:** `get_proposals`
**Method:** Tauri command
**Description:** Retrieve list of governance proposals
**Request:**
```typescript
interface GetProposalsRequest {
  status?: 'pending' | 'active' | 'succeeded' | 'defeated' | 'all';
  limit?: number;
  offset?: number;
}
```
**Response:**
```typescript
interface ProposalSummary {
  id: number;
  title: string;
  description: string;
  proposer: string;
  status: string;
  voteStart: number; // unix timestamp
  voteEnd: number; // unix timestamp
  forVotes: number;
  againstVotes: number;
  abstainVotes: number;
  quorum: number;
  createdAt: number; // unix timestamp
}
```

### Get Proposal Details
**Route:** `governance_get_proposal`
**Legacy Route:** `get_proposal`
**Method:** Tauri command
**Description:** Get detailed information about a specific proposal
**Request:**
```typescript
interface GetProposalRequest {
  proposalId: number;
}
```
**Response:**
```typescript
interface ProposalDetail {
  id: number;
  title: string;
  description: string;
  proposer: string;
  status: string;
  voteStart: number;
  voteEnd: number;
  forVotes: number;
  againstVotes: number;
  abstainVotes: number;
  quorum: number;
  createdAt: number;
  actions: ProposalAction[];
}

interface ProposalAction {
  target: string; // contract address
  value: number; // amount to send
  signature: string; // function signature
  calldata: string; // encoded parameters
}
```

### Get Proposal Votes
**Route:** `governance_get_proposal_votes`
**Legacy Route:** `get_proposal_votes`
**Method:** Tauri command
**Description:** Get votes cast on a specific proposal
**Request:**
```typescript
interface GetProposalVotesRequest {
  proposalId: number;
  limit?: number;
  offset?: number;
}
```
**Response:**
```typescript
interface VoteInfo {
  voter: string;
  proposalId: number;
  support: number; // 0=against, 1=for, 2=abstain
  votes: number;
  reason?: string;
  timestamp: number; // unix timestamp
}
```

### Get Voting Power
**Route:** `governance_get_voting_power`
**Legacy Route:** `get_voting_power`
**Method:** Tauri command
**Description:** Get voting power for an address
**Request:**
```typescript
interface GetVotingPowerRequest {
  address: string;
}
```
**Response:**
```typescript
interface VotingPowerInfo {
  address: string;
  votingPower: number; // current voting power
  delegatedPower: number; // power delegated to others
  totalPower: number; // votingPower + delegatedPower
}
```

### Get Delegations
**Route:** `governance_get_delegations`
**Legacy Route:** `get_delegations`
**Method:** Tauri command
**Description:** Get governance delegations for an address
**Request:**
```typescript
interface GetDelegationsRequest {
  address: string;
}
```
**Response:**
```typescript
interface DelegationInfo {
  delegator: string;
  delegatee: string;
  amount: number;
  timestamp: number; // unix timestamp
}
```

### Cast Vote
**Route:** `governance_cast_vote`
**Legacy Route:** `cast_vote`
**Method:** Tauri command
**Description:** Cast a vote on a proposal
**Request:**
```typescript
interface CastVoteRequest {
  proposalId: number;
  support: number; // 0=against, 1=for, 2=abstain
  votes: number;
  reason?: string;
}
```
**Response:**
```typescript
interface CastVoteResponse {
  vote: VoteInfo;
}
```

### Delegate Voting Power
**Route:** `governance_delegate_voting_power`
**Legacy Route:** `delegate_voting_power`
**Method:** Tauri command
**Description:** Delegate voting power to another address
**Request:**
```typescript
interface DelegateVotingPowerRequest {
  delegatee: string;
  amount: number;
}
```
**Response:**
```typescript
interface DelegateVotingPowerResponse {
  delegation: DelegationInfo;
}
```

### Create Proposal
**Route:** `governance_create_proposal`
**Legacy Route:** `create_proposal`
**Method:** Tauri command
**Description:** Create a new governance proposal
**Request:**
```typescript
interface CreateProposalRequest {
  title: string;
  description: string;
  actions: ProposalAction[];
}
```
**Response:**
```typescript
interface CreateProposalResponse {
  proposalId: number;
  proposal: ProposalSummary;
}
```

## Error Handling

All API endpoints should return appropriate error responses:

```typescript
interface ApiError {
  code: string; // e.g., "VALIDATION_ERROR", "INSUFFICIENT_BALANCE", "NETWORK_ERROR"
  message: string;
  details?: Record<string, any>;
}
```

## Implementation Notes

1. **Authentication**: All user-specific endpoints should validate that the request is from the authenticated wallet user
2. **Validation**: Input validation should be performed on both frontend and backend
3. **Rate Limiting**: Consider implementing rate limiting for expensive operations
4. **Caching**: Implement appropriate caching for frequently accessed data
5. **Real-time Updates**: Consider WebSocket connections for real-time updates on staking rewards, proposal status, etc.

## Testing

Each endpoint should have:
- Unit tests for business logic
- Integration tests with mock blockchain state
- Error case testing
- Authentication/authorization testing