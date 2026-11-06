import glob,re,collections
ansi=re.compile(r'\x1b\[[0-9;]*m')
shells=['bash','dash','mksh','zsh','ash','brush','gosh','yash3','bunsh','deno-task']
cnt=collections.Counter(); base=0
for f in sorted(glob.glob('/tmp/ls/results/*.txt')):
    for l in open(f,errors='replace').read().splitlines():
        l=ansi.sub('',l)
        p=l.split('\t')
        if len(p)>=13 and re.match(r'^\s*\d+$',p[0]) and re.match(r'^\s*\d+$',p[1]):
            res=p[2:12]
            if res[0]=='pass':
                base+=1
                for s,r in zip(shells,res):
                    if r=='pass': cnt[s]+=1
print('cases where bash==pass:',base)
for s in shells: print(f'{s}: {cnt[s]} ({100*cnt[s]/base:.1f}%)')
