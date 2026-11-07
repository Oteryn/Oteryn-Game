"""Retain four exact portable trophy facts for the source lava sacrifice."""
import json
from pathlib import Path
FACTS={24939:(500,'7687cb8bebcbe3f41b890c2452b2098c6151b16ad225b79852f2c7692d3a0345'),24940:(350,'97c15a1555162f9cd97aef80d73e6cd950e94d09622089b91eff7cbf2f604234'),24941:(650,'2dc38e34d66754ac8e6e8334df4c7e5c61d591e67697895d8e6ee9d69c2facdd'),24942:(800,'52bfe281bcc40adc210ee1efdeb4537c87433b495b4874060e7b172080aab8db')}
def apply_admissions(definitions):
 observations=json.loads((Path(__file__).resolve().parents[2]/'imports/tibiawiki/facts/items-stats.json').read_bytes())['records']
 known=lambda value:dict(state='KNOWN',value=value)
 stack=known(dict(stackable=known(True),stack_max=known(100)))
 for ident,(weight,digest) in FACTS.items():
  key=f'oteryn:item.tibia.i{ident}';matches=[d for d in definitions if d['identity']['key']==key];evidence=observations[key]
  if len(matches)!=1 or evidence['item_id']!=ident or len(evidence['observations'])!=1:raise ValueError('First Dragon trophy identity changed')
  observation=evidence['observations'][0]
  if observation['content_sha256']!=digest or observation['fields']['stackable'].lower()!='yes' or observation['fields']['weight']!=f'{weight/100:.2f}':raise ValueError('First Dragon trophy source changed')
  item=matches[0];old=item['semantics']['stack']
  compatible=old in ({'state':'UNKNOWN'},stack,known(dict(stackable=known(True),stack_max={'state':'UNKNOWN'})))
  if item['identity']!={'family':'Item','key':key,'revision':'definition-r1'} or item['semantics']['physical']!=known(dict(weight=known(weight),movable=known(True),pickupable=known(True))) or not compatible or (item['materializable'],item['stack_class']) not in ((False,'Unknown'),(True,'StackCapable')):raise ValueError('First Dragon trophy core facts changed')
  item['semantics']['stack']=known(dict(stackable=known(True),stack_max=known(100)));item['materializable']=True;item['stack_class']='StackCapable'
