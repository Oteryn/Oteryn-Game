"""Actual current source stock and reproducible canonical transformation."""
import json
from pathlib import Path
import unittest
import seasonal_messenger_shop as shop

class MessengerStockTests(unittest.TestCase):
 def test_actual_canonical_service_matches_current_thirteen_goods(self):
  document=json.loads((shop.ROOT/'content/world/definitions/declarations.json').read_bytes())
  service=next(row for row in document['records'] if row.get('identity',{}).get('key')==shop.KEY)
  actual={int(row['item']['key'].rsplit('i',1)[1]):row['unit_price'] for row in service['offers']}
  self.assertEqual(actual,shop.CURRENT)
 def test_current_source_normalization_is_idempotent_and_rejects_unseen_stock(self):
  rows=[{'direction':'SellToPlayer','item':{'key':f'oteryn:item.tibia.i{i}'},'unit_price':price} for i,price in shop.CURRENT.items()]
  once=shop.normalized(rows);self.assertEqual(shop.normalized(once),once)
  rows[0]['unit_price']=999
  with self.assertRaisesRegex(ValueError,'CONCURRENT_SOURCE_CONFLICT'):shop.normalized(rows)

if __name__=='__main__':unittest.main()
