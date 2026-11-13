import struct
import unittest
import build_srx3_ebb_maps as ebb

class EbbMapTests(unittest.TestCase):
    def area(self, x, y, items):
        tile = [ebb.source.otbm.NODE_TILE, bytes([0, 0])+b''.join(bytes([ebb.source.otbm.ATTR_ITEM])+struct.pack('<H', i) for i in items), []]
        return [ebb.source.otbm.NODE_TILE_AREA, struct.pack('<HHB',x,y,8), [tile]]
    def test_replacement_retains_source_outliers_and_order(self):
        rows=ebb.rows_from_nodes([self.area(1025,1004,[1949,33276]),self.area(33900,31100,[469])])
        self.assertEqual([r['source_position'] for r in rows],[[1025,1004,8],[33900,31100,8]])
        self.assertEqual([i['server_item_id'] for i in rows[0]['items']],[1949,33276])
    def test_duplicate_and_unknown_source_facts_refuse(self):
        area=self.area(1025,1004,[1949])
        with self.assertRaises(ValueError):ebb.rows_from_nodes([area,area])
        area[2][0][1]+=bytes([255])
        with self.assertRaises(ValueError):ebb.rows_from_nodes([area])
    def test_wrong_source_blob_refuses(self):
        with self.assertRaises(ValueError):ebb.verify_blob(b'changed', '0'*40)

if __name__=='__main__':unittest.main()
