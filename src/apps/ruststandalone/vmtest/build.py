def build(gen, env):
    env.m3_rust_exe(
        gen,
        out = 'vmtest',
        libs = ['isr'],
        dir = None,
        ldscript = 'tilemux',
        varAddr = False
    )
