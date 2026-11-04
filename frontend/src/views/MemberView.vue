<script setup>
/**
 * 家庭成员管理
 *
 * 展示所有成员资料，掌勺人可增删改成员、设置头像 emoji 与主题色。
 * 「我是谁」由登录身份决定，换人使用请退出后重新登录，不做应用内切换。
 */
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/api'
import { toast } from '@/composables/useToast'

const app = useAppStore()
const auth = useAuthStore()
const router = useRouter()

const editing = ref(false)
const editId = ref(null)
const saving = ref(false)
const showPicker = ref(false)
const pwdEditing = ref(false)
const pwdSaving = ref(false)
const pwdForm = reactive({ old: '', new: '', confirm: '' })

const AVATARS = ['👨', '👩', '🧑', '👦', '👧', '👴', '👵', '👶', '🧒', '👨‍🍳', '🐱', '🐶', '🐰', '🐻']
const COLORS = ['#C2703D', '#D1915A', '#C98A1E', '#A9761F', '#4F8A5B', '#6E7F96', '#A9443C', '#B85C38', '#8A4522', '#7C6A52']

const form = reactive({
  nickname: '',
  avatar_emoji: '🙂',
  theme_color: '#C2703D',
  role: 'member',
  taste_note: '',
  birthday: '',
  gender: '',
  dislikes: ''
})

const members = computed(() => app.members)
const roles = computed(() => app.meta?.member_roles || [
  { value: 'admin', label: '掌勺人' }, { value: 'member', label: '家庭成员' }, { value: 'child', label: '小朋友' }
])

/** 家庭成员管理（增删改）仅管理员可用 */
const isAdmin = computed(() => app.currentMember?.role === 'admin')

onMounted(async () => {
  if (!members.value.length) await app.loadMembers()
  if (!app.meta) await app.loadMeta()
})

function openCreate() {
  if (!isAdmin.value) { toast.warn('只有掌勺人（管理员）可以添加成员'); return }
  editId.value = null
  Object.assign(form, {
    nickname: '', avatar_emoji: '🙂', theme_color: '#C2703D', role: 'member',
    taste_note: '', birthday: '', gender: '', dislikes: ''
  })
  editing.value = true
}

function openEdit(m) {
  if (!isAdmin.value) { toast.warn('只有掌勺人（管理员）可以编辑成员'); return }
  editId.value = m.id
  Object.assign(form, {
    nickname: m.nickname,
    avatar_emoji: m.avatar_emoji,
    theme_color: m.theme_color,
    role: m.role,
    taste_note: m.taste_note || '',
    birthday: m.birthday || '',
    gender: m.gender || '',
    dislikes: m.dislikes || ''
  })
  editing.value = true
}

async function save() {
  if (!form.nickname.trim()) { toast.warn('请填写昵称'); return }
  saving.value = true
  try {
    // 空字符串 → null（后端语义：null=清空，缺省=不修改，编辑表单整包回传）
    const payload = {
      ...form,
      nickname: form.nickname.trim(),
      birthday: form.birthday || null,
      gender: form.gender || null,
      dislikes: (form.dislikes || '').trim() || null
    }
    if (editId.value) {
      await api.members.update(editId.value, payload)
      toast.success('成员已更新')
    } else {
      await api.members.create(payload)
      toast.success('成员已添加')
    }
    await app.loadMembers()
    editing.value = false
  } catch (e) {
    toast.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function removeMember(m) {
  if (!isAdmin.value) { toast.warn('只有掌勺人（管理员）可以移除成员'); return }
  if (m.id === app.currentMemberId) { toast.warn('不能删除当前登录成员，请先退出再换人登录'); return }
  if (!confirm(`确定要移除「${m.nickname}」吗？`)) return
  try {
    await api.members.remove(m.id)
    await app.loadMembers()
    toast.success('已处理')
  } catch (e) {
    toast.error(e.message || '删除失败')
  }
}

/** 打开改密码弹层（仅当前登录成员可改自己的密码） */
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
  auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="page page--flat members">
    <div class="page-body">
      <header class="m-head">
        <div class="m-head-row">
          <div class="grow">
            <h1 class="m-title">👨‍👩‍👧 家庭成员</h1>
            <p class="m-sub">家人的资料与口味备注，增删改由掌勺人管理；换人使用请退出后重新登录</p>
          </div>
          <button v-if="app.currentMember" class="logout-btn" @click="logout">退出登录</button>
        </div>
      </header>

      <!-- 成员网格 -->
      <div class="member-grid stagger">
        <div
          v-for="m in members"
          :key="m.id"
          class="mcard"
          :class="{ active: m.id === app.currentMemberId, inactive: m.is_active === false }"
          :style="{ '--mc': m.theme_color }"
        >
          <span v-if="m.id === app.currentMemberId" class="mcard-flag">当前</span>
          <div class="mcard-avatar">
            <img v-if="m.avatar_url" :src="m.avatar_url" alt="" class="mcard-avatar-img" />
            <template v-else>{{ m.avatar_emoji }}</template>
          </div>
          <p class="mcard-name ellipsis">{{ m.nickname }}</p>
          <p class="mcard-role">{{ (roles.find(r => r.value === m.role)?.label) || m.role }}</p>
          <p v-if="m.taste_note" class="mcard-note ellipsis-2">{{ m.taste_note }}</p>
          <div class="mcard-ops">
            <button v-if="m.id === app.currentMemberId" class="mcard-op" @click.stop="openPwd">改密码</button>
            <button v-if="isAdmin" class="mcard-op" @click.stop="openEdit(m)">编辑</button>
            <button v-if="isAdmin" class="mcard-op mcard-op--del" @click.stop="removeMember(m)">移除</button>
          </div>
        </div>

        <button v-if="isAdmin" class="mcard mcard--add" @click="openCreate">
          <span class="mcard-add-icon">＋</span>
          <span class="mcard-add-text">添加成员</span>
        </button>
      </div>
    </div>

    <!-- 改密码弹层 -->
    <transition name="sheet">
      <div v-if="pwdEditing" class="sheet-mask" @click.self="pwdEditing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>修改登录密码</h2>
            <button class="sheet-close" @click="pwdEditing = false">✕</button>
          </div>

          <div class="sheet-body">
            <div class="field">
              <label class="field-label">当前成员</label>
              <p class="pwd-who">
                <span class="pwd-who-avatar" :style="{ background: app.currentMember?.theme_color }">{{ app.currentMember?.avatar_emoji }}</span>
                {{ app.currentMember?.nickname }}
              </p>
            </div>

            <div class="field">
              <label class="field-label">原密码</label>
              <input v-model="pwdForm.old" type="password" class="input" placeholder="初始密码 123456" autocomplete="current-password" />
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

    <!-- 编辑弹层 -->
    <transition name="sheet">
      <div v-if="editing" class="sheet-mask" @click.self="editing = false">
        <div class="sheet">
          <div class="sheet-head">
            <h2>{{ editId ? '编辑成员' : '添加成员' }}</h2>
            <button class="sheet-close" @click="editing = false">✕</button>
          </div>

          <div class="sheet-body">
            <!-- 预览 -->
            <div class="preview">
              <div class="preview-avatar" :style="{ background: form.theme_color }">{{ form.avatar_emoji }}</div>
              <p class="preview-name">{{ form.nickname || '昵称' }}</p>
            </div>

            <div class="field">
              <label class="field-label">昵称 <span class="req">*</span></label>
              <input v-model="form.nickname" class="input" placeholder="如：爸爸 / 宝宝 / 奶奶" maxlength="32" />
            </div>

            <div class="field">
              <label class="field-label">头像</label>
              <div class="picker">
                <button v-for="a in AVATARS" :key="a" class="pick" :class="{ on: form.avatar_emoji === a }" @click="form.avatar_emoji = a">{{ a }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">主题色</label>
              <div class="picker">
                <button v-for="c in COLORS" :key="c" class="pick pick--color" :class="{ on: form.theme_color === c }" :style="{ background: c }" @click="form.theme_color = c" />
              </div>
            </div>

            <div class="field">
              <label class="field-label">家庭角色</label>
              <div class="seg">
                <button v-for="r in roles" :key="r.value" class="seg-btn" :class="{ on: form.role === r.value }" @click="form.role = r.value">{{ r.label }}</button>
              </div>
            </div>

            <div class="field">
              <label class="field-label">口味备注</label>
              <input v-model="form.taste_note" class="input" placeholder="如：不吃香菜、少辣" maxlength="100" />
            </div>
          </div>

          <div class="sheet-foot">
            <button class="btn btn--ghost grow" @click="editing = false">取消</button>
            <button class="btn btn--primary grow" :disabled="saving" @click="save">{{ saving ? '保存中…' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.m-head { padding: calc(var(--safe-top) + 56px) 0 18px; }
.m-head-row { display: flex; align-items: flex-start; gap: var(--gap-md); }
.m-title { font-size: 24px; font-weight: 800; letter-spacing: -.02em; }
.m-sub { font-size: 13px; color: var(--text-3); margin-top: 5px; line-height: 1.5; }

.logout-btn {
  flex: none;
  margin-top: 4px;
  padding: 7px 15px;
  border-radius: var(--r-full);
  border: 1.5px solid var(--border-strong);
  background: var(--surface);
  color: var(--text-2);
  font-size: 12.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease);
  -webkit-tap-highlight-color: transparent;
}
.logout-btn:active { transform: scale(.95); color: var(--danger); border-color: var(--danger-bg); background: var(--danger-bg); }

.pwd-who { display: flex; align-items: center; gap: 9px; font-size: 15px; font-weight: 700; color: var(--text-1); }
.pwd-who-avatar { width: 34px; height: 34px; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: 19px; }

.member-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
@media (min-width: 700px) { .member-grid { grid-template-columns: repeat(3, 1fr); } }

.mcard { position: relative; background: var(--surface); border-radius: var(--r-lg); padding: 18px 14px; box-shadow: var(--sh-sm); border: 2px solid transparent; text-align: center; transition: all var(--dur) var(--ease); }
.mcard:active { transform: scale(.97); }
.mcard.active { border-color: var(--mc); box-shadow: 0 0 0 3px color-mix(in srgb, var(--mc) 18%, transparent), var(--sh-md); }
.mcard.inactive { opacity: .5; }
.mcard-flag { position: absolute; top: 10px; right: 10px; font-size: 10px; font-weight: 700; color: #fff; background: var(--mc); padding: 2px 8px; border-radius: var(--r-full); }
.mcard-avatar { width: 58px; height: 58px; margin: 0 auto 10px; border-radius: 50%; background: color-mix(in srgb, var(--mc) 16%, white); display: flex; align-items: center; justify-content: center; font-size: 30px; overflow: hidden; }
.mcard-avatar-img { width: 100%; height: 100%; object-fit: cover; display: block; }
.mcard-name { font-size: 16px; font-weight: 700; }
.mcard-role { font-size: 11.5px; color: var(--mc); font-weight: 600; margin-top: 2px; }
.mcard-note { font-size: 11.5px; color: var(--text-3); margin-top: 8px; line-height: 1.4; min-height: 30px; }
.mcard-ops { display: flex; gap: 6px; justify-content: center; margin-top: 12px; }
.mcard-op { padding: 5px 14px; border-radius: var(--r-full); border: 1px solid var(--border-strong); background: var(--surface); color: var(--text-2); font-size: 12px; cursor: pointer; font-family: inherit; }
.mcard-op--del { color: var(--danger); border-color: var(--danger-bg); }

.mcard--add { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; border: 2px dashed var(--border-strong); background: transparent; box-shadow: none; min-height: 160px; }
.mcard-add-icon { font-size: 30px; color: var(--text-4); }
.mcard-add-text { font-size: 13px; color: var(--text-3); font-weight: 600; }

/* 弹层 */
.sheet-mask { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,.4); display: flex; align-items: flex-end; justify-content: center; }
.sheet { width: 100%; max-width: 560px; background: var(--surface); border-radius: var(--r-2xl) var(--r-2xl) 0 0; max-height: 90vh; display: flex; flex-direction: column; box-shadow: var(--sh-xl); }
.sheet-head { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 12px; }
.sheet-head h2 { font-size: 18px; font-weight: 700; }
.sheet-close { width: 32px; height: 32px; border-radius: 50%; border: none; background: var(--bg-subtle); color: var(--text-3); font-size: 15px; cursor: pointer; }
.sheet-body { padding: 0 20px; overflow-y: auto; flex: 1; }
.sheet-foot { display: flex; gap: 10px; padding: 14px 20px calc(14px + var(--safe-bottom)); border-top: 1px solid var(--border); }

.preview { display: flex; flex-direction: column; align-items: center; gap: 8px; margin-bottom: 20px; }
.preview-avatar { width: 68px; height: 68px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 36px; }
.preview-name { font-size: 16px; font-weight: 700; }

.picker { display: flex; flex-wrap: wrap; gap: 8px; }
.pick { width: 44px; height: 44px; border-radius: var(--r-sm); border: 2px solid transparent; background: var(--bg-subtle); font-size: 22px; cursor: pointer; transition: all var(--dur-fast); }
.pick.on { border-color: var(--brand-500); background: var(--brand-50); transform: scale(1.05); }
.pick--color { border-radius: 50%; }
.pick--color.on { box-shadow: 0 0 0 2px #fff inset, 0 0 0 3px var(--brand-500); }

.seg { display: flex; gap: 6px; background: var(--bg-subtle); padding: 4px; border-radius: var(--r-sm); }
.seg-btn { flex: 1; padding: 9px; border: none; background: transparent; border-radius: var(--r-xs); font-size: 13px; font-weight: 600; color: var(--text-2); cursor: pointer; font-family: inherit; }
.seg-btn.on { background: var(--surface); color: var(--brand-600); box-shadow: var(--sh-xs); }

.sheet-enter-active, .sheet-leave-active { transition: opacity var(--dur); }
.sheet-enter-active .sheet, .sheet-leave-active .sheet { transition: transform var(--dur) var(--ease-out); }
.sheet-enter-from, .sheet-leave-to { opacity: 0; }
.sheet-enter-from .sheet, .sheet-leave-to .sheet { transform: translateY(100%); }
</style>
