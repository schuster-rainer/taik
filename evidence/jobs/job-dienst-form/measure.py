import subprocess, time, os, sys, json, shutil, statistics
S=sys.argv[1]; binary=os.path.abspath('target/release/taik-seed'); uid=os.getuid()
def trial(state, n, gap):
    out=[]
    seed=os.path.join(S,'od-seed.run')
    if not os.path.exists(seed): subprocess.run([binary,'seed',seed],check=True,capture_output=True)
    for i in range(n):
        time.sleep(gap)
        t=time.time()
        job=subprocess.run([binary,'queue',state,seed],check=True,capture_output=True,text=True).stdout.strip()
        while True:
            runs=sorted(f for f in os.listdir(job) if f.startswith('round-') and f.endswith('.run'))
            if runs and 'advance=false' in open(os.path.join(job,runs[-1])).read(): break
            if time.time()-t>30: out.append(None); break
            time.sleep(0.001)
        else: continue
        if out and out[-1] is None: continue
        out.append(round((time.time()-t)*1000,1))
    return out
def launch(label, state, extra):
    plist=os.path.join(S,label+'.plist')
    open(plist,'w').write(f'''<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>Label</key><string>{label}</string>
<key>ProcessType</key><string>Interactive</string>
<key>WorkingDirectory</key><string>{os.getcwd()}</string>{extra}</dict></plist>''')
    subprocess.run(['launchctl','bootstrap',f'gui/{uid}',plist],check=True)
def stop(label): subprocess.run(['launchctl','bootout',f'gui/{uid}/{label}'],capture_output=True)
res={}
for name,extra_fn,gap in [
 ('persistent', lambda st: f'<key>ProgramArguments</key><array><string>{binary}</string><string>serve</string><string>{st}</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><true/>', 1),
 ('on_demand_gap_1s', lambda st: f'<key>ProgramArguments</key><array><string>{binary}</string><string>drain</string><string>{st}</string></array><key>WatchPaths</key><array><string>{st}/jobs</string></array><key>ThrottleInterval</key><integer>0</integer>', 1),
 ('on_demand_gap_12s', lambda st: f'<key>ProgramArguments</key><array><string>{binary}</string><string>drain</string><string>{st}</string></array><key>WatchPaths</key><array><string>{st}/jobs</string></array><key>ThrottleInterval</key><integer>0</integer>', 12),
]:
    st=os.path.join(S,'od-'+name); os.makedirs(os.path.join(st,'jobs'),exist_ok=True)
    label='local.taik.measure.'+name.replace('_','-')
    launch(label, st, extra_fn(st)); time.sleep(2)
    try: res[name]=trial(st, 6, gap)
    finally: stop(label)
    print(name, res[name], flush=True)
json.dump(res, open(os.path.join(S,'ondemand.json'),'w'))
