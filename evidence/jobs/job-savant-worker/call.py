import json, subprocess, sys, time, os
S=sys.argv[1]; name=sys.argv[2]; brief=open(sys.argv[3]).read()
prompt=("You are the code worker embedded in TAIK. Return one complete standalone HTML document, no fences or explanation.\n\nWORK ORDER:\n%s\n\nACCEPTED SOURCE TO BUILD ON:\n(empty)" % brief)
req={"model":"subq-savant-1.4","messages":[{"role":"user","content":prompt}],"max_tokens":40000,"reasoning_effort":"low"}
open(S+'/savant-req.json','w').write(json.dumps(req))
t=time.time()
r=subprocess.run([S+'/subq.sh','https://api.subq.ai/v1/chat/completions','-d','@'+S+'/savant-req.json'],capture_output=True,text=True,env={**os.environ,'SUBQ_TIMEOUT':'300'})
ms=round((time.time()-t)*1000)
try: d=json.loads(r.stdout)
except Exception: print('bad response',r.stdout[:300],r.stderr[:200]); sys.exit(1)
if 'choices' not in d: print('error',json.dumps(d)[:400]); sys.exit(1)
text=d['choices'][0]['message'].get('content') or ''
text=text.strip()
if text.startswith('```'): text=text.split('\n',1)[1].rsplit('\n```',1)[0]
open(S+'/savant-%s.txt'%name,'w').write(text+'\n')
u=d.get('usage',{})
cost=u.get('prompt_tokens',0)*0.10/1e6+u.get('completion_tokens',0)*0.50/1e6
json.dump({'name':name,'ms':ms,'finish':d['choices'][0].get('finish_reason'),'usage':u,'cost_usd':round(cost,6),'model':d.get('model')},open(S+'/savant-%s.json'%name,'w'))
print(name,ms,'ms',d['choices'][0].get('finish_reason'),u.get('prompt_tokens'),u.get('completion_tokens'),'cost',round(cost,6),len(text),'bytes')
