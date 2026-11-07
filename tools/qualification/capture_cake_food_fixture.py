#!/usr/bin/env python3
"""Seal first actual native Cake USE/chat transcript, without constructing bytes."""
import argparse,hashlib,json
from pathlib import Path
from seasonal_capture_fields import metadata, fields, envelopes

def materialize(v,artifact):
 if v.get('schema')!='OTERYN_CAKE_FOOD_NATIVE_CHAT_CAPTURE/v1':raise ValueError('actual Cake Food schema')
 s=v['source']
 if s.get('chat7') is not True or s.get('ambient288') is not False:raise ValueError('ordinary chat7 alone')
 if s['observer_runtime_cell']!=[33303,32066,7] or s['b3_sha256']!='2798be76221d6dc00695258d46d1eb375a64e804a7a84265014e2af0c679b05d':raise ValueError('actual source observer/B3')
 if (s['command_id'],s['command_type'])!=(1,2):raise ValueError('actual first ordinary USE')
 b=bytes.fromhex(v['frames_hex'])
 if not b or len(b)>4*1024*1024 or hashlib.sha256(b).hexdigest()!=v['frames_sha256']:raise ValueError('actual frames SHA')
 accepted=next(envelopes(b))
 if accepted.get(1)!=[2]:raise ValueError('actual ServerAccepted origin')
 selected=set(fields(accepted[4][0]).get(10,[]))
 if not isinstance(s.get('scripted386'),bool) or 7 not in selected or 288 in selected or (386 in selected)!=s['scripted386']:raise ValueError('actual scripted/legacy negotiation')
 if v.get('mmm_count')!=1:raise ValueError('actual fresh source speech count')
 if hashlib.sha256(artifact).hexdigest()!=s['client_content_generation']:raise ValueError('independently compiled production client pin')
 v['last_server_sequence'],_=metadata(b)
 cursor=0;boundary=None
 while cursor<len(b):
  if cursor+4>len(b):raise ValueError('actual frame prefix')
  size=int.from_bytes(b[cursor:cursor+4],'big');end=cursor+4+size
  if not size or end>len(b):raise ValueError('actual frame bounds')
  envelope=fields(b[cursor+4:end]);cursor=end
  if envelope.get(1)==[13]:boundary=cursor;break
 if boundary is None or boundary==len(b):raise ValueError('actual initial snapshot boundary')
 for prefix,raw in (('initial_frames',b[:boundary]),('reply_frames',b[boundary:])):
  v[prefix+'_hex']=raw.hex();v[prefix+'_sha256']=hashlib.sha256(raw).hexdigest()
 if bytes.fromhex(v['initial_frames_hex'])+bytes.fromhex(v['reply_frames_hex'])!=b:raise ValueError('causal split changed original transcript')
 s['command_payload_sha256']=hashlib.sha256(bytes.fromhex(s['command_payload_hex'])).hexdigest()
 return (json.dumps(v,sort_keys=True,indent=2)+'\n').encode()
def verify_native_request(capture, actual_hex):
 raw=bytes.fromhex(capture['frames_hex'])
 if hashlib.sha256(raw).hexdigest()!=capture['frames_sha256']:raise ValueError('actual server transcript SHA')
 accepted=next(envelopes(raw))
 if accepted.get(1)!=[2]:raise ValueError('actual acceptance origin')
 generation=fields(accepted[4][0])[4]
 actual=bytes.fromhex(actual_hex)
 if not actual or len(actual)>1048576:raise ValueError('bounded actual client command')
 envelope=fields(actual)
 if set(envelope)!={1,2,4} or envelope[1]!=[7] or envelope[2]!=generation:raise ValueError('native request type/generation differs from actual admitted session')
 command=fields(envelope[4][0]);source=capture['source']
 if set(command)!={1,2,4} or command[1]!=[source['command_id']] or command[2]!=[source['command_type']] or command[4]!=[bytes.fromhex(source['command_payload_hex'])]:raise ValueError('native request id/type/payload differs from actual source USE')
 print('actual native Food command id/type/payload verified')

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path,nargs='?');p.add_argument('--client-artifact',type=Path);p.add_argument('--check',action='store_true');p.add_argument('--verify-native-request');a=p.parse_args();capture=json.loads(a.capture.read_bytes())
 if a.verify_native_request is not None:
  verify_native_request(capture,a.verify_native_request);return
 if a.output is None or a.client_artifact is None:raise ValueError('normalization requires output and independently compiled client artifact')
 data=materialize(capture,a.client_artifact.read_bytes())
 if a.check:
  if a.output.read_bytes()!=data:raise ValueError('actual Cake Food fixture drift')
 else:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_bytes(data)
 print('actual Cake Food fixture '+('verified' if a.check else 'generated'))
if __name__=='__main__':main()
