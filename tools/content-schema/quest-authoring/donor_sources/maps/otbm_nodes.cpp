#include <zlib.h>
#include <array>
#include <cstdint>
#include <iostream>
#include <stdexcept>
#include <vector>
// Exact structural OTBM projection. Payload ranges refer to the decoded file
// stream and retain escaped bytes. Records are emitted in node-close order.
struct Node {uint32_t id,parent,start,begin,end;unsigned type;bool children;};
int main(int argc,char**argv){
 if(argc!=3)return 2;
 gzFile in=gzopen(argv[1],"rb"),out=gzopen(argv[2],"wb1");
 if(!in||!out){std::cerr<<"open failed\n";return 2;}
 uint64_t offset=0;uint32_t count=0;unsigned roots=0,maxdepth=0;
 std::array<uint64_t,256> types{};std::vector<Node> stack;
 auto read=[&](){int c=gzgetc(in);if(c>=0)++offset;return c;};
 auto required=[&](){int c=read();if(c<0)throw std::runtime_error("truncated source");return c;};
 try{
  for(int i=0;i<4;++i)required();
  int c;
  while((c=read())>=0){
   if(offset>UINT32_MAX)throw std::runtime_error("decoded stream exceeds codec uint32");
   uint32_t pos=offset-1;
   if(c==0xfe){
    if(stack.empty()){if(++roots!=1)throw std::runtime_error("multiple roots");}
    else if(!stack.back().children){stack.back().end=pos;stack.back().children=true;}
    uint32_t parent=stack.empty()?UINT32_MAX:stack.back().id;
    int type=required();if(type==0xfd)type=required();
    stack.push_back({count++,parent,pos,pos+1,0,(unsigned)type,false});
    if(stack.size()>maxdepth)maxdepth=stack.size();
   }else if(c==0xff){
    if(stack.empty())throw std::runtime_error("unmatched node end");
    Node n=stack.back();stack.pop_back();if(!n.children)n.end=pos;
    uint32_t fields[]={n.id,n.parent,n.start,(uint32_t)offset,n.begin,n.end};
    unsigned char bytes[24];for(int f=0;f<6;++f)for(int j=0;j<4;++j)bytes[f*4+j]=(fields[f]>>(8*j))&255;
    if(gzwrite(out,bytes,24)!=24)throw std::runtime_error("index write failed");
    ++types[n.type];
   }else{
    if(stack.empty())throw std::runtime_error("data outside root");
    if(stack.back().children)throw std::runtime_error("properties after children");
    if(c==0xfd)required();
   }
  }
  int error=0;gzerror(in,&error);if(error!=Z_OK&&error!=Z_STREAM_END)throw std::runtime_error("source decompression error");
  if(!stack.empty()||roots!=1)throw std::runtime_error("unclosed or absent root");
  if(gzclose(in)!=Z_OK||gzclose(out)!=Z_OK)throw std::runtime_error("stream close failed");
  std::cout<<"{\"node_count\":"<<count<<",\"decoded_bytes\":"<<offset<<",\"max_depth\":"<<maxdepth<<",\"types\":{";
  bool first=true;for(int i=0;i<256;++i)if(types[i]){if(!first)std::cout<<",";first=false;std::cout<<"\""<<i<<"\":"<<types[i];}
  std::cout<<"}}\n";return 0;
 }catch(const std::exception&e){gzclose(in);gzclose(out);std::cerr<<e.what()<<"\n";return 1;}
}
