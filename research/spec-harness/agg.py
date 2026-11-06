import glob,re,collections,sys,json
ansi=re.compile(r'\x1b\[[0-9;]*m')
tot=collections.defaultdict(collections.Counter)
perfile={}
bad=[]
for f in sorted(glob.glob('/tmp/ls/results/*.txt')):
    lines=[ansi.sub('',l) for l in open(f,errors='replace').read().splitlines()]
    hdr=None
    for i,l in enumerate(lines):
        if l.startswith('\t\t') and 'bash' in l and 'deno-task' in l:
            hdr=[h for h in l.split('\t') if h]; start=i
    if hdr is None: bad.append(f); continue
    pf={}
    for l in lines[start+1:]:
        parts=l.split('\t')
        if len(parts)<3 or not parts[1]: continue
        key=parts[1]; vals=[p for p in parts[2:] if p!='']
        for h,v in zip(hdr,vals):
            tot[h][key]+=int(v); pf.setdefault(h,collections.Counter())[key]+=int(v)
    perfile[f.split('/')[-1][:-4]]=pf
print('bad',bad)
shells=['bash','dash','mksh','zsh','ash','brush','gosh','yash3','bunsh','deno-task']
for s in shells:
    c=tot[s]; t=c['total']
    acc=c['pass']+c['ok']+c['N-I']+c['BUG']
    print(f"| {s} | {t} | {c['pass']} ({100*c['pass']/t:.1f}%) | {acc} ({100*acc/t:.1f}%) | {c['FAIL']} | {c['TIME']} |")
json.dump(perfile,open('/tmp/ls/perfile.json','w'))
