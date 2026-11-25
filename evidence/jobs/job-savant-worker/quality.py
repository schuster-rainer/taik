import subprocess, os, time, json, sys
S=sys.argv[1]
T=[("\tprint 1 / 0;","ERROR 1:10 DIV_ZERO\n",1),("print 1;\n","1\n",0),("print 1\n","ERROR 2:1 SYNTAX\n",1),
("let a = 2; let b = a * a * a; print b - -b;","16\n",0),("print 2 * -3;","-6\n",0),("print - - -4;","-4\n",0),
("print 9223372036854775807;","9223372036854775807\n",0),("print -9223372036854775807 - 1;","-9223372036854775808\n",0),
("print 00000000000000000000009;","9\n",0),("print 99999999999999999999;","ERROR 1:7 LITERAL_RANGE\n",1),
("print 1 @ 2 / 0;","ERROR 1:9 BAD_CHAR\n",1),("print 1 / 0; print $;","ERROR 1:20 BAD_CHAR\n",1),
("let letx = 1; print letx;","1\n",0),("print (((((1)))));","1\n",0),("print 1 +;","ERROR 1:10 SYNTAX\n",1),
("let x 1;","ERROR 1:7 SYNTAX\n",1),("x = 1;","ERROR 1:1 SYNTAX\n",1),("print 5 % 3 * 2;","4\n",0),("print 2 - 3 * 4 % 5;","0\n",0),
("let a = 1; print a; let a = 2; print a;","1\nERROR 1:25 DUPLICATE\n",1),
("print "+"("*2000+"1"+")"*2000+";","1\n",0),("print "+"-"*3001+"1;","-1\n",0)]
big="".join("let v%d = %d * 3 + 1; print v%d %% 7;\n"%(i,i,i) for i in range(60000))
out={}
for name in ['minicalc-opus-1','minicalc-opus-2','minicalc-savant-1','minicalc-savant-2']:
    src=open(f'evidence/jobs/job-{name}/round-01.html').read().split('\n',1)[1]
    d=f'{S}/q-{name}'; os.makedirs(d,exist_ok=True); open(d+'/main.rs','w').write(src)
    c=subprocess.run(['rustc','--edition=2021','-O','main.rs','-o','prog'],cwd=d,capture_output=True,text=True)
    warnings=c.stderr.count('warning:')
    fails=[]
    for i,(inp,exp,code) in enumerate(T):
        try:
            r=subprocess.run([d+'/prog'],input=inp,capture_output=True,text=True,timeout=5)
            if r.stdout!=exp or r.returncode!=code: fails.append((i+1, (r.stdout[:40], r.returncode)))
        except subprocess.TimeoutExpired: fails.append((i+1,'timeout'))
    t=time.time(); r=subprocess.run([d+'/prog'],input=big,capture_output=True,text=True,timeout=30); ms=round((time.time()-t)*1000)
    ok=r.returncode==0 and r.stdout.count('\n')==60000
    out[name]={'lines':src.count('\n'),'bytes':len(src),'rustc_warnings':warnings,'heldout_passed':len(T)-len(fails),'heldout_total':len(T),'failed_cases':fails,'big_program_ms':ms,'big_program_ok':ok,
      'unwrap_or_expect':src.count('.unwrap()')+src.count('.expect('),'comments':sum(1 for l in src.split('\n') if l.strip().startswith('//'))}
    print(name,out[name])
json.dump(out,open(S+'/quality.json','w'),indent=1)
