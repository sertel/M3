dirs = [
    'allocator',
    'asciiplay',
    'bench',
    'bsdutils',
    'coreutils',
    'cppnettests',
    'disktest',
    'dosattack',
    'evilcompute',
    'faulter',
    'filterchain',
    'hashmuxtests',
    'hello',
    'info',
    'libctest',
    'msgchan',
    'netechoserver',
    'noop',
    'parchksum',
    'ping',
    'plasma',
    'queue',
    'rusthello',
    'rustnettests',
    'ruststandalone',
    'rustunittests',
    'shell',
    'spammer',
    'standalone',
    'timertest',
    'unittests',
]

def build(gen, env):
    for d in dirs:
        env.sub_build(gen, d)
