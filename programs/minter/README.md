# mint-wrapper

Mints tokens to authorized accounts.

## Description

The `mint-wrapper` program wraps a token mint and authorizes specific accounts to mint tokens up to given allowances.

The `mint-wrapper` also enforces a hard cap of a token.

Thishis should be used to prevent the `Rewarder` from over-issuing tokens.

This can also be used for several other use cases, including but not limited to:

- Allocating funds to a DAO
- Allocating team lockups

## Roadmap

Future improvements may include:

- Allowing transfer of the `mint_authority` to a different address
- Remove/ close account minter
