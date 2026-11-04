<script setup>
/**
 * 个人中心
 *
 * 首页头像入口落点：展示当前登录成员，支持
 * 更换头像（相册上传 / emoji）、修改登录密码、退出登录。
 * 换人登录请退出后重新进家门（不做应用内切换成员）。
 */
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/api'
import { compressToJpeg } from '@/utils/image'
import { toast } from '@/composables/useToast'

const app = useAppStore()
const auth = useAuthStore()
const router = useRouter()

const AVATARS = ['👨', '👩', '🧑', '👦', '👧', '👴', '👵', '👶', '🧒', '👨‍🍳', '🐱', '🐶', '🐰', '🐻']

const avatarEditing = ref(false)
const avatarSaving = ref(false)
/** 'photo' = 用图片头像；'emoji' = 用 emoji 头像 */
const avatarMode = ref('emoji')
const avatarPick = ref('')
/** 本地预览用的 dataURL（未保存前） */
const photoPreview = ref('')
/** 文件选择 input */
const fileInput = ref(null)

const pwdEditing = ref(false)
const pwdSaving = ref(false)
const pwdForm = reactive({ old: '', new: '', confirm: '' })

const member = computed(() => app.currentMember)
const roleLabel = computed(() => {
  const r = app.meta?.member_roles?.find((x) => x.value === member.value?.role)
  return r?.label || member.value?.role || '家庭成员'
})
/** 头像展示：有图片用图片，否则 emoji */
const avatarImg = computed(() => member.value?.avatar_url || '')

onMounted(async () => {
  if (!app.members.length) await app.loadMembers()
  if (!app.meta) await app.loadMeta()
})

/** 打开换头像弹层 */
function openAvatar() {
  avatarMode.value = member.value?.avatar_url ? 'photo' : 'emoji'
  avatarPick.value = member.value?.avatar_emoji || '🙂'
  photoPreview.value = member.value?.avatar_url || ''
  avatarEditing.value = true
}

/** 从相册选图（input file） */
function onPickFile(e) {
  const file = e.target.files?.[0]
  e.target.value = '' // 允许重选同一张
  if (!file) return
  if (!file.type.startsWith('image/')) { toast.warn('请选择图片文件'); return }
  compressToJpeg(file, { maxSize: 512, quality: 0.85 })
    .then((dataUrl) => {
      photoPreview.value = dataUrl
      avatarMode.value = 'photo'
    })
    .catch(() => toast.error('图片读取失败，请换一张'))
}

/** 选 emoji 头像（同时放弃图片） */
function pickEmoji(a) {
  avatarPick.value = a
  avatarMode.value = 'emoji'
  photoPreview.value = ''
}

async function saveAvatar() {
  if (!member.value) return
  avatarSaving.value = true
  try {
    if (avatarMode.value === 'photo') {
      if (!photoPreview.value) { toast.warn('请先从相册选一张图'); return }
      // 已是服务端地址（没换过图）则无需重传
      let url = photoPreview.value
      if (!url.startsWith('/api/uploads/')) {
        const res = await api.uploads.create(url)
        url = res.url
      }
      await api.members.update(member.value.id, { avatar_url: url })
    } else {
      if (!avatarPick.value) { toast.warn('请选一个头像'); return }
      // 显式清空 avatar_url（回落 emoji）
      await api.members.update(member.value.id, {
        avatar_emoji: avatarPick.value,
        avatar_url: null
      })
    }
    await app.loadMembers()
    toast.success('头像已更换')
    avatarEditing.value = false
  } catch (e) {
    toast.error(e.message || '更换失败，请重试')
  } finally {
    avatarSaving.value = false
  }
}

function openPwd() {
  Object.assign(pwdForm, { old: '', new: '', confirm: '' })
  pwdEditing.value = true
}

function savePwd() {
  if (!pwdForm.old) { toast.warn('请输入原密码'); return }
  if (pwdForm.new.length < 4) { toast.warn('新密码至少 4 位'); return }
  if (pwdForm.new !== pwdForm.confirm) { toast.warn('两次输入的新密码不一致'); return }
  pwdSaving.value = true
  auth.changePassword(app.currentMemberId, pwdForm.old, pwdForm.new).then((res) => {
    pwdSaving.value = false
    if (!res.ok) { toast.error(res.error || '修改失败'); return }
    toast.success('密码已更新，下次登录用新密码')
    pwdEditing.value = false
  }).catch(() => {
    pwdSaving.value = false
    toast.error('修改失败，请重试')
  })
}

function logout() {
  if (!confirm('确定要退出登录吗？')) return
  auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="page page--flat profile">
    <div class="page-body">
      <!-- 个人信息卡 -->
      <section class="hero stagger">
        <button
          class="hero-avatar"
          :style="{ background: member?.theme_color || 'var(--brand-500)' }"
          aria-label="更换头像"
          @click="openAvatar"
        >
          <img v-if="avatarImg" :src="avatarImg" alt="" class="hero-avatar-img" />
          <span v-else>{{ member?.avatar_emoji || '👨‍👩‍👧' }}</span>
          <i class="hero-avatar-edit">📷</i>
        </button>
        <p class="hero-name">{{ member?.nickname || '家人' }}</p>
        <p class="hero-role">{{ roleLabel }}</p>
      </section>

      <!-- 功能列表 -->
      <section class="menu stagger">
        <button class="menu-item" @click="openAvatar">
          <span class="menu-icon">🖼️</span>
          <span class="menu-text">
            <span class="menu-title">更换头像</span>
            <span class="menu-sub">从相册上传，或选一个喜欢的 emoji</span>
          </span>
          <span class="menu-arrow">›</span>
        </button>

        <button class="menu-item" @click="openPwd">
          <span class="menu-icon">🔑</span>
          <span class="menu-text">
            <span class="menu-title">修改密码</span>
            <span class="menu-sub">定期换一换，家门更安心</span>
          </span>
          <span class="menu-arrow">›</span>
        </button>

        <button class="menu-item" @click="router.push('/profile/version')">
          <span class="menu-icon">🏷️</span>
          <span class="menu-text">
            <span class="menu-title">系统版本</span>
            <span class="menu-sub">当前版本与历次更新内容</span>
          </span>
          <span class="menu-arrow">›</span>
        </button>

        <button class="menu-item menu-item--danger" @click="logout">
          <span class="menu-icon">🚪</span>
          <span class="menu-text">
            <span class="menu-title">退出登录</span>
            <span class="menu-sub">换家人使用请退出后重新进家门</span>
          </span>
          <span class="menu-arrow">›</span>
        </button>
      </section>

      <p class="foot-note">数据保存在本机与家庭空间，只在家里人之间共享</p>
    </div>

    <!-- 更换头像弹层 -->
    <transition name="sheet">
      <div v-if="avatarEditing" class="sheet-mask" @click.self="avatarEditing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>更换头像</h2>
            <button class="sheet-close" @click="avatarEditing = false">✕</button>
          </div>
          <div class="sheet-body">
            <div class="preview">
              <div class="preview-avatar" :style="{ background: member?.theme_color || 'var(--brand-500)' }">
                <img v-if="avatarMode === 'photo' && photoPreview" :src="photoPreview" alt="" class="preview-avatar-img" />
                <template v-else>{{ avatarPick }}</template>
              </div>
              <p class="preview-name">{{ member?.nickname }}</p>
            </div>

            <!-- 相册上传 -->
            <input ref="fileInput" type="file" accept="image/*" class="file-hide" @change="onPickFile" />
            <button class="upload-btn" @click="fileInput?.click()">
              <span class="upload-btn-icon">📷</span>
              从相册上传
            </button>
            <p class="upload-hint">支持 JPG / PNG，上传前会自动压缩</p>

            <!-- emoji 头像 -->
            <p class="pick-label">或选一个 emoji 头像</p>
            <div class="picker">
              <button
                v-for="a in AVATARS"
                :key="a"
                class="pick"
                :class="{ on: avatarMode === 'emoji' && avatarPick === a }"
                @click="pickEmoji(a)"
              >{{ a }}</button>
            </div>
          </div>
          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="avatarEditing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="avatarSaving" @click="saveAvatar">{{ avatarSaving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 修改密码弹层 -->
    <transition name="sheet">
      <div v-if="pwdEditing" class="sheet-mask" @click.self="pwdEditing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>修改登录密码</h2>
            <button class="sheet-close" @click="pwdEditing = false">✕</button>
          </div>
          <div class="sheet-body">
            <div class="field">
              <label class="field-label">原密码</label>
              <input v-model="pwdForm.old" type="password" class="input" placeholder="请输入原密码" autocomplete="current-password" />
            </div>
            <div class="field">
              <label class="field-label">新密码（至少 4 位）</label>
              <input v-model="pwdForm.new" type="password" class="input" placeholder="请输入新密码" autocomplete="new-password" />
            </div>
            <div class="field">
              <label class="field-label">确认新密码</label>
              <input v-model="pwdForm.confirm" type="password" class="input" placeholder="再输入一次新密码" autocomplete="new-password" />
            </div>
          </div>
          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="pwdEditing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="pwdSaving" @click="savePwd">{{ pwdSaving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: calc(var(--safe-top) + 48px) 0 28px;
}
.hero-avatar {
  position: relative;
  width: 92px;
  height: 92px;
  border-radius: 50%;
  border: 4px solid var(--surface);
  box-shadow: var(--sh-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 46px;
  overflow: visible;
  cursor: pointer;
  transition: transform var(--dur-fast) var(--ease);
  -webkit-tap-highlight-color: transparent;
}
.hero-avatar:active { transform: scale(.93); }
.hero-avatar-img {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
  display: block;
}
.hero-avatar-edit {
  position: absolute;
  right: -2px;
  bottom: -2px;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--surface);
  box-shadow: var(--sh-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-style: normal;
}
.hero-name { font-size: 22px; font-weight: 800; letter-spacing: -.02em; margin-top: 14px; }
.hero-role {
  margin-top: 6px;
  padding: 3px 12px;
  border-radius: var(--r-full);
  background: var(--brand-50);
  color: var(--brand-600);
  font-size: 12px;
  font-weight: 700;
}

.menu {
  background: var(--surface);
  border-radius: var(--r-lg);
  box-shadow: var(--sh-sm);
  overflow: hidden;
}
.menu-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 15px 16px;
  border: none;
  border-bottom: 1px solid var(--border);
  background: transparent;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
  transition: background var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.menu-item:last-child { border-bottom: none; }
.menu-item:active { background: var(--brand-50); }
.menu-item--danger:active { background: var(--danger-bg); }
.menu-item--danger .menu-title { color: var(--danger); }

.menu-icon {
  flex: none;
  width: 40px;
  height: 40px;
  border-radius: var(--r-sm);
  background: var(--brand-50);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}
.menu-item--danger .menu-icon { background: var(--danger-bg); }

.menu-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.menu-title { font-size: 15.5px; font-weight: 700; color: var(--text-1); }
.menu-sub { font-size: 12px; color: var(--text-3); }
.menu-arrow { flex: none; font-size: 20px; color: var(--text-4); line-height: 1; }

.foot-note {
  margin-top: 18px;
  text-align: center;
  font-size: 11.5px;
  color: var(--text-4);
  line-height: 1.6;
}

/* 弹层 */
.sheet-mask {
  position: fixed;
  inset: 0;
  z-index: 100;
  background: rgba(0, 0, 0, .4);
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.sheet {
  width: 100%;
  max-width: 560px;
  background: var(--surface);
  border-radius: var(--r-2xl) var(--r-2xl) 0 0;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--sh-xl);
}
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: var(--bg-subtle);
  color: var(--text-3);
  font-size: 15px;
  cursor: pointer;
}
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot {
  display: flex;
  gap: 10px;
  padding: 14px 20px calc(14px + var(--safe-bottom));
  border-top: 1px solid var(--border);
}

.preview { display: flex; flex-direction: column; align-items: center; gap: 8px; margin-bottom: 16px; }
.preview-avatar {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 38px;
  overflow: hidden;
}
.preview-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.preview-name { font-size: 16px; font-weight: 700; }

.file-hide { display: none; }
.upload-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 13px;
  border-radius: var(--r-md);
  border: 1.5px dashed var(--brand-300);
  background: var(--brand-50);
  color: var(--brand-600);
  font-size: 15px;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  transition: all var(--dur-fast);
  -webkit-tap-highlight-color: transparent;
}
.upload-btn:active { transform: scale(.97); background: var(--brand-100); }
.upload-btn-icon { font-size: 18px; }
.upload-hint {
  margin-top: 6px;
  text-align: center;
  font-size: 11.5px;
  color: var(--text-4);
}

.pick-label {
  margin: 16px 0 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-3);
}

.picker { display: flex; flex-wrap: wrap; gap: 8px; padding-bottom: 8px; }
.pick {
  width: 46px;
  height: 46px;
  border-radius: var(--r-sm);
  border: 2px solid transparent;
  background: var(--bg-subtle);
  font-size: 23px;
  cursor: pointer;
  transition: all var(--dur-fast);
}
.pick.on {
  border-color: var(--brand-500);
  background: var(--brand-50);
  transform: scale(1.05);
}

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }

@media (min-width: 700px) {
  .menu { max-width: 560px; margin: 0 auto; }
}
</style>
