#!/usr/bin/env bash

POLL_INTERVAL=1
CONFIRMATION_TIME=10

# Fail instantly if anything goes wrong and log executed commands
set -euxo pipefail

# Clean up before exit
function cleanup {
  pkill server
  pkill ln_gateway
  pkill lightningd
  pkill bitcoind
}
trap cleanup EXIT

SRC_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )/.." &> /dev/null && pwd )"

# Define temporary directories to not overwrite manually created config if run locally
TMP_DIR="$(mktemp -d)"
echo "Working in $TMP_DIR"
LN1_DIR="$TMP_DIR/ln1"
mkdir $LN1_DIR
LN2_DIR="$TMP_DIR/ln2"
mkdir $LN2_DIR
BTC_DIR="$TMP_DIR/btc"
mkdir $BTC_DIR
CFG_DIR="$TMP_DIR/cfg"
mkdir $CFG_DIR

# Build all executables
cd $SRC_DIR
cargo build --release
BIN_DIR="$SRC_DIR/target/release"

# Generate federation, gateway and client config
$BIN_DIR/configgen -- $CFG_DIR 4 4000 5000 1000 10000 100000 1000000 10000000
$BIN_DIR/gw_configgen -- $CFG_DIR "$LN1_DIR/regtest/lightning-rpc"

# Start bitcoind and wait for it to become ready
bitcoind -regtest -fallbackfee=0.0004 -txindex -server -rpcuser=bitcoin -rpcpassword=bitcoin -datadir=$BTC_DIR &
BTC_CLIENT="bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin"
until [ "$($BTC_CLIENT getblockchaininfo | jq -r '.chain')" == "regtest" ]; do
  sleep $POLL_INTERVAL
done

# Initialize wallet and get ourselves some money
$BTC_CLIENT createwallet main
function mine_blocks() {
    PEG_IN_ADDR="$($BTC_CLIENT getnewaddress)"
    $BTC_CLIENT generatetoaddress $1 $PEG_IN_ADDR
}
mine_blocks 120

# Start lightning nodes
lightningd --network regtest --bitcoin-rpcuser=bitcoin --bitcoin-rpcpassword=bitcoin --lightning-dir=$LN1_DIR --addr=127.0.0.1:9000 &
lightningd --network regtest --bitcoin-rpcuser=bitcoin --bitcoin-rpcpassword=bitcoin --lightning-dir=$LN2_DIR --addr=127.0.0.1:9001 &
until [ -e $LN1_DIR/regtest/lightning-rpc ]; do
    sleep $POLL_INTERVAL
done
until [ -e $LN2_DIR/regtest/lightning-rpc ]; do
    sleep $POLL_INTERVAL
done
LN1="lightning-cli --network regtest --lightning-dir=$LN1_DIR"
LN2="lightning-cli --network regtest --lightning-dir=$LN2_DIR"

# Open channel
LN_ADDR="$($LN1 newaddr | jq -r '.bech32')"
$BTC_CLIENT sendtoaddress $LN_ADDR 1
mine_blocks 10
LN2_PUB_KEY="$($LN2 getinfo | jq -r '.id')"
$LN1 connect $LN2_PUB_KEY@127.0.0.1:9001
until $LN1 fundchannel $LN2_PUB_KEY 0.1btc; do sleep $POLL_INTERVAL; done
mine_blocks 10

# FIXME: make db path configurable to avoid cd-ing here
# Start the federation members inside the temporary directory
cd $TMP_DIR
for ((ID=0; ID<4; ID++)); do
  echo "starting mint $ID"
  ($BIN_DIR/server $CFG_DIR/server-$ID.json 2>&1 | sed -e "s/^/mint $ID: /" ) &
done
MINT_CLIENT="$BIN_DIR/mint-client $CFG_DIR"

function await_block_sync() {
  EXPECTED_BLOCK_HEIGHT="$(( $($BTC_CLIENT getblockchaininfo | jq -r '.blocks') - $CONFIRMATION_TIME ))"
  for ((ID=0; ID<4; ID++)); do
    MINT_API_URL="http://127.0.0.1:500$ID"
    until [ "$(curl $MINT_API_URL/wallet/block_height)" == "$EXPECTED_BLOCK_HEIGHT" ]; do
      sleep $POLL_INTERVAL
    done
  done
}
await_block_sync

# Start LN gateway

$BIN_DIR/ln_gateway $CFG_DIR &

#### BEGIN TESTS ####
# peg in
PEG_IN_ADDR="$($MINT_CLIENT peg-in-address)"
TX_ID="$($BTC_CLIENT sendtoaddress $PEG_IN_ADDR 0.00099999)"

# Confirm peg-in
mine_blocks 11
await_block_sync
TXOUT_PROOF="$($BTC_CLIENT gettxoutproof "[\"$TX_ID\"]")"
TRANSACTION="$($BTC_CLIENT getrawtransaction $TX_ID)"
$MINT_CLIENT peg-in "$TXOUT_PROOF" "$TRANSACTION"
$MINT_CLIENT fetch

# reissue
TOKENS=$($MINT_CLIENT spend 42000)
$MINT_CLIENT reissue $TOKENS
$MINT_CLIENT fetch

# peg out
PEG_OUT_ADDR="$($BTC_CLIENT getnewaddress)"
$MINT_CLIENT peg-out $PEG_OUT_ADDR "500 sat"
sleep 5 # wait for tx to be included
mine_blocks 120
await_block_sync
sleep 15
mine_blocks 10
RECEIVED=$($BTC_CLIENT getreceivedbyaddress $PEG_OUT_ADDR)
[[ "$RECEIVED" = "0.00000500" ]]

# outgoing lightning
INVOICE="$($LN2 invoice 100000 test test 1m | jq -r '.bolt11')"
$MINT_CLIENT ln-pay $INVOICE
INVOICE_RESULT="$($LN2 waitinvoice test)"
INVOICE_STATUS="$(echo $INVOICE_RESULT | jq -r '.status')"
[[ "$INVOICE_STATUS" = "paid" ]]

$BIN_DIR/clientd $CFG_DIR &
RPC="http://127.0.0.1:8081/rpc"
sleep 15
#CLIENTD
#JSON-RPC Specification
#TODO: Notification: no ID means the client does not want any response (reissue for example)
#rpc call of non-existent method:
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "doesnotexist","params": null,"id": 1}');
#RES_ERROR=$(echo $RES | jq '.error');  #should contain an rpc error -32601
#RES_RES=$(echo  $RES | jq '.result'); #should be null
#[[ $RES_RES = "null" ]]
#[[ $(echo $RES_ERROR | jq '.code') = "-32601" ]]
#rpc call with invalid JSON:
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","info": "info","params": null,"id": 1'); # the '}' is missing
#RES_ERROR=$(echo $RES | jq '.error'); #should contain an rpc error -32700
#RES_RES=$(echo  $RES | jq '.result'); #should be null
#[[ "$RES_RES" = "null" ]]
#[[ $(echo $RES_ERROR | jq '.code') = "-32700" ]]
#TODO: rpc call Batch, invalid JSON:
#TODO: rpc call with invalid Batch:
#TODO: rpc call Batch
#Requests: all methods (also add option to use positional (array) params ?)
# method: info
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "info","params": null,"id": 1}')
#TODO check if response is ok
# method: pending
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "pending","params": null,"id": 1}')
#TODO check if response is ok
# method: events
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "events","params": null,"id": 1}')
#TODO check if response is ok
# method: pegin_address
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "pegin_address","params": null,"id": 1}')
#TODO check if response is ok
# method: pegin
#PEG_IN_ADDR="$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "pegin_address","params": null,"id": 1}' | jq -r '.result.PegInAddress.pegin_address')"
#TX_ID="$($BTC_CLIENT sendtoaddress $PEG_IN_ADDR 0.00099999)"
# Confirm peg-in
#mine_blocks 11
#await_block_sync
#TXOUT_PROOF="$($BTC_CLIENT gettxoutproof "[\"$TX_ID\"]")"
#TRANSACTION="$($BTC_CLIENT getrawtransaction $TX_ID)"
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method": "pegin","params": {"txout_proof":'\""$TXOUT_PROOF"\"',"transaction":'\""$TRANSACTION"\"'},"id": 1}')
#TODO check if response is ok
# method: spend, reissue_validate
#SPEND_RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method": "spend","params": 42000,"id": 1}' | jq '.result.Spend.token')
#REISSUE_RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "reissue_validate","params": '"$TOKENS"',"id": 1}')
#TODO check if response is ok
# method: pegout
#PEG_OUT_ADDR="$($BTC_CLIENT getnewaddress)"
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc": "2.0","method": "pegout","params": {"address": '\""$PEG_OUT_ADDR"\"',"amount": 500},"id": 1}')
#sleep 5 # wait for tx to be included
#mine_blocks 120
#await_block_sync
#sleep 15
#mine_blocks 10
#RECEIVED=$($BTC_CLIENT getreceivedbyaddress $PEG_OUT_ADDR)
#[[ "$RECEIVED" = "0.00000500" ]]
# method: lnpay
#INVOICE="$(lightning-cli --network regtest --lightning-dir=ln2 invoice 100000 test test 10m | jq '.bolt11')"
#RES=$(curl -X POST $RPC -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method": "lnpay","params": {"bolt11":'"$INVOICE"'},"id": 1}')
#INVOICE_RESULT="$(lightning-cli --network regtest --lightning-dir=ln2 waitinvoice test)"
#echo $INVOICE_RESULT;
#INVOICE_STATUS="$(echo $INVOICE_RESULT | jq -r '.status')"
#echo $INVOICE_STATUS;
#[[ "$INVOICE_STATUS" = "paid" ]]