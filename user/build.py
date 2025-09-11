import os

# 定义基地址和步长，步长需要确保大于应用程序的最大尺寸
base_address = 0x80400000
step = 0x20000
# 定义链接脚本路径
linker = 'src/linker.ld'

# 获取所有应用程序
app_id = 0
apps = os.listdir('src/bin')
apps.sort()

# 编译每个应用程序
for app in apps:
    # 首先获得程序的名称，去除扩展名
    app = app[:app.find('.')]
    lines = []
    lines_before = []
    # 读取链接脚本并修改起始地址，然后写回文件
    with open(linker, 'r') as f:
        for line in f.readlines():
            lines_before.append(line)
            # 替换起始地址，针对原始链接脚本中的0x80400000
            # 替换为base_address + step * app_id
            # 因此每个应用程序的起始地址相差step
            # 内存中也具备了同时具有所有应用程序的代码的条件
            # （因为每个 app 的内存区域不重叠）
            line = line.replace(hex(base_address), hex(base_address+step*app_id))
            lines.append(line)
    with open(linker, 'w+') as f:
        f.writelines(lines)
    # 修改完成链接脚本后，编译应用程序
    os.system('cargo build --bin %s --release' % app)
    print('[build.py] application %s start with address %s' %(app, hex(base_address+step*app_id)))
    with open(linker, 'w+') as f:
        f.writelines(lines_before)
    app_id = app_id + 1
