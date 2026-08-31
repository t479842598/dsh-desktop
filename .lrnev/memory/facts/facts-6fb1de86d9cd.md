id: facts-6fb1de86d9cd
category: facts
scope: global
source: "dsh-desktop progress.md 2026-09-01 dsh alpha.2 升级回滚排查"
created: "2026-08-31T16:33:20.000Z"
reference_count: 0
---

# facts-6fb1de86d9cd

dsh 0.1.2-alpha.2 新增官方 launch-token 认证层后，与自定义 ds-auths-plugin 密码门认证栈叠加冲突：带 token 访问 3080 官方层通过（303），但带 cookie 访问首页时 ds-auths-plugin fallback 抛 `AUTH_INTERNAL_ERROR` 500 → 客户端白屏。实测 rc.2（0.1.1）无官方认证层、ds-auths-plugin 本地回环直接放行，整套正常可用。结论：本项目客户端保持 dsh 0.1.1-rc.2 为稳定基线；桌面壳需在 healthy_dsh 中把带 dsh 特征的 401 视为健康（避免误杀 launchd 常驻服务），且本地导航应从 dsh 启动日志解析 `dsh web: ...?token=` URL（见 dsh.rs launch_token_url / is_dsh_healthy_response）。
