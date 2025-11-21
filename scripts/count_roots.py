with open("./scripts/yuchai.txt", "rt") as f:
    yuchai = f.read()

result = []
for line in yuchai.splitlines():
    if not line: continue
    [zi, roots] = line.split('\t')
    roots = roots.split(' ')
    result.append((zi, len(roots)))

with open("count.txt", "wt") as f:
    out = ""
    for zi, count in result:
        out += f"{zi}\t{count}\n"
    f.write(out)
