import io,pathlib,tarfile,tempfile,unittest
from donor_sources.validate_archive import verify_archive
class ArchiveControls(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.path=pathlib.Path(self.tmp.name)/'source.tar.gz'
 def write(self,names,link=False):
  with tarfile.open(self.path,'w:gz') as t:
   for name in names:
    m=tarfile.TarInfo(name);m.size=1
    if link:m.type=tarfile.SYMTYPE;m.linkname='../outside';m.size=0;t.addfile(m)
    else:t.addfile(m,io.BytesIO(b'x'))
 def test_archive_traversal_rejected(self):
  self.write(['../outside'])
  with self.assertRaisesRegex(ValueError,'unsafe donor path'):verify_archive(self.path)
 def test_duplicate_members_rejected(self):
  self.write(['x','x'])
  with self.assertRaisesRegex(ValueError,'duplicate archive'):verify_archive(self.path)
 def test_symlink_not_source_bytes(self):
  self.write(['link'],link=True)
  with self.assertRaisesRegex(ValueError,'ordinary source file'):verify_archive(self.path)
 def test_bad_archive_digest_rejected(self):
  self.write(['x'])
  with self.assertRaisesRegex(ValueError,'digest differs'):verify_archive(self.path,'0'*64)
if __name__=='__main__':unittest.main()
