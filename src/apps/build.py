dirs = [
    'allocator',
    'asciiplay',
    'bench',
    'coreutils',
    'cppnettests',
    'disktest',
    'dosattack',
    'evilcompute',
    'faulter',
    'filterchain',
    'float',
    'hello',
    'libctest',
    'msgchan',
    'netechoserver',
    'noop',
    'parchksum',
    'plasma',
    'queue',
    'rdwr',
    'rusthello',
    'rustnettests',
    'ruststandalone',
    'rustunittests',
    'shell',
    'standalone',
    'timertest',
    'unittests',
]

def build(gen, env):
    for d in dirs:
        env.sub_build(gen, d)
