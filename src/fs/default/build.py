def build(gen, env):
    if env['TGT'] == 'host':
        blocks = 160 * 1024
    else:
        blocks = 32 * 1024
    env.build_fs(gen, out = 'default.img', dir = '.', blocks = blocks, inodes = 512)
