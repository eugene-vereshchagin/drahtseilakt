# Band levels of a generated role against its original. usage: python3 diag.py ROLE ...
import os, sys, numpy as np, scipy.signal as ss, scipy.io.wavfile as wf, json
H = os.path.dirname(os.path.abspath(__file__))
from analyse import load, D
bands=[30,60,120,250,500,1000,2000,4000,8000,11000,16000]
sc=json.load(open(os.path.join(H, 'scores.json')))
def ob(x,sr):
    f,P=ss.welch(x,sr,nperseg=min(2048,len(x)))
    v=np.array([10*np.log10(P[(f>=a)&(f<b)].sum()+1e-14) for a,b in zip(bands,bands[1:])]); return v-v[:9].max()
for role in sys.argv[1:]:
    o=sc[role]['original']; osr,ox=load(D+o+'.wav'); gsr,g=wf.read(os.path.join(H, 'generated', f'{role}.wav')); g=g/32768
    print(role,o); print('  orig',' '.join(f'{u:4.0f}' for u in ob(ox,osr)[:9])); print('  gen ',' '.join(f'{u:4.0f}' for u in ob(g,gsr)[:9]))
