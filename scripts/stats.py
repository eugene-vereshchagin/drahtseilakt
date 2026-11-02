#!/usr/bin/env python3
# The numbers on docs/drahtseilakt/stats.md: lines of code and docs, commits, and the AI's tokens and working hours
# read from Claude Code's session logs (~/.claude/projects/<project>/, passed as the argument).
import datetime as dt, glob, json, os, subprocess, sys

def lines(pattern):
    files = glob.glob(pattern, recursive=True)
    return sum(sum(1 for _ in open(f, errors='replace')) for f in files), len(files)

def active_hours(stamps, gap_min=15):
    stamps = sorted(stamps)
    return sum(d for d in ((b - a).total_seconds() for a, b in zip(stamps, stamps[1:])) if d < gap_min * 60) / 3600

def logs(root):
    tokens = dict(input=0, cache_write=0, cache_read=0, output=0); calls = {}; seen = set(); main, per_file = [], 0.0
    for f in glob.glob(os.path.join(root, '**', '*.jsonl'), recursive=True):
        sub = '/subagents/' in f or os.path.basename(f).startswith('agent-'); own = []
        for line in open(f, errors='replace'):
            try:
                e = json.loads(line)
            except ValueError:
                continue
            if e.get('timestamp'):
                t = dt.datetime.fromisoformat(e['timestamp'].replace('Z', '+00:00')); own.append(t)
                if not sub:
                    main.append(t)
            m = e.get('message')
            if e.get('type') == 'assistant' and isinstance(m, dict) and m.get('usage'):
                key = (m.get('id'), e.get('requestId'))
                if key in seen:
                    continue
                seen.add(key); u = m['usage']; calls[m.get('model')] = calls.get(m.get('model'), 0) + 1
                tokens['input'] += u.get('input_tokens', 0); tokens['output'] += u.get('output_tokens', 0)
                tokens['cache_write'] += u.get('cache_creation_input_tokens', 0)
                tokens['cache_read'] += u.get('cache_read_input_tokens', 0)
        per_file += active_hours(own)
    return tokens, calls, active_hours(main), per_file, (min(main), max(main)) if main else None

if __name__ == '__main__':
    for name, pat in [('rust', 'crates/**/*.rs'), ('wgsl', 'crates/**/*.wgsl'), ('C reference', 'src/**/*.[ch]'),
                      ('docs en', 'docs/drahtseilakt/*.md'), ('docs ru', 'docs/ru/drahtseilakt/*.md'),
                      ('docs de', 'docs/de/drahtseilakt/*.md'), ('docs zh', 'docs/zh/drahtseilakt/*.md')]:
        print(name, *lines(pat))
    grep = subprocess.run(['grep', '-r', '-c', r'#\[test\]', 'crates'], capture_output=True, text=True).stdout
    print('tests', sum(int(line.rsplit(':', 1)[1]) for line in grep.split()))
    print('commits', subprocess.run(['git', 'rev-list', '--count', 'HEAD'], capture_output=True, text=True).stdout.strip())
    if len(sys.argv) > 1:
        tokens, calls, main_h, agent_h, span = logs(sys.argv[1])
        print('tokens', tokens, 'total', sum(tokens.values()))
        print('api calls', calls)
        print('hours: session', round(main_h, 1), 'all agents summed', round(agent_h, 1), 'span', span)
