#!/usr/bin/env node
// Fixed-route adapter for the saved modern Oteryn-Game environment.
// Uses the installed desktop client's RPC contract; this is not an official CLI API.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';

// The saved environment config id is account-specific; it comes from the
// environment, never from the repository.
const CONFIG_ID = process.env.OTERYN_CODEX_ENV_CONFIG_ID ?? '';
if (!CONFIG_ID) { process.stderr.write('ERROR: set OTERYN_CODEX_ENV_CONFIG_ID (see oteryn-waves references/delegation.md).\n'); process.exit(1); }
const RPC_URL = 'https://codex-cloud-backend.chatgpt.com/grpc';
const args = process.argv.slice(2);
const command = args.shift();
const opt = name => { const n=args.indexOf(name); return n<0?undefined:args[n+1]; };
const authPath=path.join(process.env.CODEX_HOME || path.join(os.homedir(),'.codex'),'auth.json');
const stateDir=path.join(process.env.CODEX_HOME || path.join(os.homedir(),'.codex'),'oteryn-modern-tasks');
let auth;

async function rpc(service,method,request) {
  if(!auth) {
    const saved=JSON.parse(fs.readFileSync(authPath,'utf8'));
    if(!saved.tokens?.access_token || !saved.tokens?.account_id) throw Error('Existing ChatGPT Codex login required. Run codex login.');
    auth=saved.tokens;
  }
  const response=await fetch(RPC_URL,{
    method:'POST',redirect:'error',signal:AbortSignal.timeout(60000),
    headers:{'Content-Type':'application/json','Accept':'application/json',
      'Authorization':`Bearer ${auth.access_token}`,'ChatGPT-Account-Id':auth.account_id,
      'X-OpenAI-Product-Sku':'codex'},
    body:JSON.stringify({method:`/oaiproto.codex_app_server_backend.${service}/${method}`,request})
  });
  if(!response.ok) {
    let code,message; try { ({code,message}=await response.json()); } catch {}
    const detail=typeof message==='string'?message.replaceAll(auth.access_token,'[redacted]').slice(0,500):'';
    throw Error(`Modern Cloud RPC ${method} failed: HTTP ${response.status}, code ${code??'unknown'}: ${detail}. No legacy fallback and no automatic mutation retry.`);
  }
  return response.json();
}
function print(value) { process.stdout.write(JSON.stringify(value,null,2)+'\n'); }
async function checkedConfig() {
  const {config}=await rpc('EnvironmentConfigService','ReadConfig',{environment_config_id:CONFIG_ID});
  if(config?.id!==CONFIG_ID || config?.name!=='Oteryn-Game') throw Error('Saved environment identity mismatch. Submission stopped.');
  if(config.status!=='ready' || !config.latest_ready_version_id)throw Error('Saved environment is not ready. Submission stopped.');
  return config;
}
function stateFile(id) {
  if(!/^[a-zA-Z0-9_-]+$/.test(id)) throw Error('Invalid thread ID');
  return path.join(stateDir,id+'.json');
}

try {
  if(command==='check') {
    const c=await checkedConfig();
    print({name:c.name,environmentConfigId:c.id,status:c.status,versionId:c.version_id,latestReadyVersionId:c.latest_ready_version_id,route:'modern',legacyFallback:false});
  } else if(command==='submit') {
    const promptFile=opt('--prompt-file');
    if(!promptFile) throw Error('Usage: node oteryn-cloud.mjs submit --prompt-file task.txt [--title title] [--branch ref]');
    const prompt=fs.readFileSync(promptFile,'utf8').trim();
    if(!prompt) throw Error('Prompt file is empty.');
    const config=await checkedConfig();
    const branch=opt('--branch');
    const managed={environmentConfigId:CONFIG_ID,cwd:'/workspace'};
    if(branch){
      const repository=(config.repositories?.values??config.repositories??[]).find(r=>(r.repository_id??r.id)==='github-1338291140');
      if(!repository)throw Error('Expected saved repository was not found. No branch override applied.');
      managed.overrides={repositories:{values:[{repositoryId:repository.repository_id??repository.id,ref:branch}]}};
    }
    const dispatchedAt=new Date().toISOString();
    const started=await rpc('ThreadService','Prewarm',{
      cwd:'/workspace',
      environments:{values:[{managed}]},
      sandbox:'SANDBOX_MODE_WORKSPACE_WRITE',
      runtimeWorkspaceRoots:{values:['/workspace']},
      threadSource:'oteryn-modern-adapter',
      deferredEnvironment:true
    });
    const threadId=started.thread?.id;
    if(!threadId) throw Error('Modern service did not return a thread ID. Do not resubmit blindly.');
    fs.mkdirSync(stateDir,{recursive:true,mode:0o700});
    const state={threadId,environmentConfigId:CONFIG_ID,branch:branch??null,dispatchedAt,clientUserMessageId:crypto.randomUUID(),admitted:false};
    fs.writeFileSync(stateFile(threadId),JSON.stringify(state,null,2),{mode:0o600});
    print({threadId,environmentConfigId:CONFIG_ID,dispatchedAt,stateFile:stateFile(threadId)});
    if(opt('--title')) await rpc('ThreadService','SetThreadName',{threadId,name:opt('--title')});
    const admitted=await rpc('ThreadService','AddUserMessage',{
      threadId,clientUserMessageId:state.clientUserMessageId,
      sandboxPolicy:{workspaceWrite:{writableRoots:{values:['/workspace']},networkAccess:true}},
      input:{values:[{text:{text:prompt}}]},
      deferredEnvironment:true
    });
    state.admitted=true;
    state.admittedAt=new Date().toISOString();
    fs.writeFileSync(stateFile(threadId),JSON.stringify(state,null,2),{mode:0o600});
    print({threadId,admitted:true,admissionOutcome:admitted.admissionOutcome,admittedAt:state.admittedAt});
  } else if(command==='status') {
    const threadId=args[0];
    stateFile(threadId??'');
    const result=await rpc('ThreadService','ReadThread',{threadId});
    const turns=await rpc('ThreadService','ListTurns',{threadId,limit:10,sortDirection:'SORT_DIRECTION_DESC',itemsView:{string3:'TURN_ITEMS_VIEW_STRING3_FULL'}});
    print({...result,recentTurns:turns});
  } else if(command==='send') {
    const threadId=args[0];
    stateFile(threadId??'');
    const file=opt('--prompt-file');
    if(!file)throw Error('send requires --prompt-file');
    const prompt=fs.readFileSync(file,'utf8').trim();
    if(!prompt)throw Error('Prompt file is empty');
    const thread=await rpc('ThreadService','ReadThread',{threadId});
    if(!thread.thread?.environments?.values?.some(e=>e.environmentConfigId===CONFIG_ID))throw Error('Thread is not attached to the fixed modern environment');
    const result=await rpc('ThreadService','AddUserMessage',{
      threadId,clientUserMessageId:crypto.randomUUID(),
      sandboxPolicy:{workspaceWrite:{writableRoots:{values:['/workspace']},networkAccess:true}},
      input:{values:[{text:{text:prompt}}]},
      deferredEnvironment:true
    });
    print({threadId,admitted:true,admissionOutcome:result.admissionOutcome});
  } else if(command==='poll') {
    // Short read-only poll: no mutations, no retries.
    const threadId=args[0];
    stateFile(threadId??'');
    const thread=await rpc('ThreadService','ReadThread',{threadId});
    let execution,environments;
    try { execution=await rpc('ThreadService','GetThreadExecutionState',{threadId}); } catch(e) { execution={error:e.message}; }
    const ids=(thread.thread?.environments?.values??[]).map(e=>e.environmentId).filter(Boolean);
    try {
      const registry=await rpc('EnvironmentService','ReadEnvironmentStatuses',{environmentIds:{values:ids}});
      environments=(registry.data?.values??[]).map(v=>({status:v.status,phase:v.observations?.phase,failure:!!v.observations?.failure}));
    } catch(e) { environments={error:e.message}; }
    print({threadId,status:thread.thread?.status,execution,environments});
  } else if(command==='list') {
    const result=await rpc('ThreadService','ListThreads',{limit:30,sortKey:'THREAD_SORT_KEY_CREATED_AT',sortDirection:'SORT_DIRECTION_DESC'});
    print({threads:(result.data?.values??[]).filter(t=>t.environments?.values?.some(e=>e.environmentConfigId===CONFIG_ID)).map(t=>({id:t.id,name:t.name,status:t.status,createdAt:t.createdAt,environments:t.environments})),nextCursor:result.nextCursor});
  } else {
    throw Error('Usage: node oteryn-cloud.mjs check | submit --prompt-file task.txt [--title title] [--branch ref] | status THREAD_ID | poll THREAD_ID | send THREAD_ID --prompt-file followup.txt | list');
  }
} catch(error) {
  process.stderr.write(`ERROR: ${error.message}\n`);
  process.exitCode=1;
}
