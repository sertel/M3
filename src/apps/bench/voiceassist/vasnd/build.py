def build(gen, env):
    env = env.clone()

    env['CPPPATH'] += ['src/libs/flac/include']

    env.m3_exe(gen, out = 'vasnd', ins = ['encoder.cc', 'vasnd.cc'], libs = ['flac'])