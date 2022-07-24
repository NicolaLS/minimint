#!/usr/bin/env bash
# Runs a CLI-based integration test

set -euxo pipefail
export RUST_LOG=info

source ./scripts/lib.sh
source ./scripts/build.sh
source ./scripts/setup-tests.sh
./scripts/start-fed.sh
export PEG_IN_AMOUNT=99999
./scripts/pegin.sh $PEG_IN_AMOUNT 1 # peg in gateway

#### BEGIN TESTS ####
$FM_CLIENTD $FM_CFG_DIR &
echo $! >> $FM_PID_FILE
await_server_on_port 8081

#peg-in user
ADDR=$($FM_CLI new-pegin-address | jq -r '.Success.pegin_address')
TX_ID="$($FM_BTC_CLIENT sendtoaddress $ADDR 0.00099999)"

# Now we "wait" for confirmations
$FM_BTC_CLIENT generatetoaddress 11 "$($FM_BTC_CLIENT getnewaddress)"
await_block_sync

TXOUT_PROOF="$($FM_BTC_CLIENT gettxoutproof "[\"$TX_ID\"]")"
TRANSACTION="$($FM_BTC_CLIENT getrawtransaction $TX_ID)"

#finaly test the clientd cli peg-in
$FM_CLI peg-in $TXOUT_PROOF $TRANSACTION

#spend and reissue
TOKENS=$($FM_MINT_CLIENT spend 42 | jq -r '.Success.pegin_address')

