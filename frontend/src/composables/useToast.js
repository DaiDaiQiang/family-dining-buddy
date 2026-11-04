import { ref, h, Transition, TransitionGroup } from 'vue'

/**
 * 轻量全局提示（不引第三方库）
 *
 * 用法：
 *   toast.success('已下单')
 *   toast.error('失败')
 *   toast.info('...')
 */
const items = ref([])
let seed = 0

const ICONS = {
  success: '✓',
  error: '✕',
  info: 'i',
  warn: '!'
}

function push(message, type = 'info', duration = 2200) {
  const id = ++seed
  items.value.push({ id, message: String(message ?? ''), type })
  setTimeout(() => {
    items.value = items.value.filter((i) => i.id !== id)
  }, duration)
  return id
}

export const toast = {
  success: (m, d) => push(m, 'success', d),
  error: (m, d) => push(m, 'error', d ?? 2800),
  info: (m, d) => push(m, 'info', d),
  warn: (m, d) => push(m, 'warn', d ?? 2800)
}

/** 提示容器组件：挂在 App.vue 根部 */
export const ToastHost = {
  name: 'ToastHost',
  setup() {
    return () =>
      h(
        'div',
        { class: 'toast-host', role: 'status', 'aria-live': 'polite' },
        [
          h(
            TransitionGroup,
            { name: 'toast' },
            {
              default: () =>
                items.value.map((it) =>
                  h('div', { key: it.id, class: `toast toast--${it.type}` }, [
                    h('span', { class: 'toast-icon' }, ICONS[it.type] || 'i'),
                    h('span', { class: 'toast-text' }, it.message)
                  ])
                )
            }
          )
        ]
      )
  }
}

export function useToast() {
  return { toast, items }
}

export default toast
