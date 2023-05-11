#!/usr/bin/python3

import sys
import requests
import json

blockNumber =requests.get('https://blockscout.com/poa/xdai/api?module=block&action=eth_block_number').json()['result']

query="""{
    id
    organization
    outgoing { limit limitPercentage canSendToAddress userAddress }
    incoming { limit limitPercentage canSendToAddress userAddress }
    balances { amount token { id owner { id } } }
}""".replace('\n', ' ')

#API='https://graph.circles.garden/subgraphs/name/CirclesUBI/circles-subgraph'
API='https://api.thegraph.com/subgraphs/name/circlesubi/circles-ubi'


lastID = 0
count = 1000
safes = []
success = False
print("Block number: %d" % int(blockNumber, 0))
for totalTries in range(500):
    print("ID: %s" % lastID)
    result = requests.post(API, data='{"query":"{ safes( orderBy: id, first: %d, where: { id_gt: \\"%s\\" } ) %s }"}' % (count, lastID, query)).json()
    if 'errors' in result:
        continue
    if 'data' not in result or 'safes' not in result['data'] or len(result['data']['safes']) == 0:
        print("Last response:")
        print(result)
        success = True
        break
    print("Got %d safes..." % len(result['data']['safes']))
    safes += result['data']['safes']
    lastID = result['data']['safes'][-1]['id']
if not success:
    print("Too many failures or requests.")
    sys.exit(1)


json.dump({'blockNumber': blockNumber, 'safes': safes}, open('safes.json', 'w'))
