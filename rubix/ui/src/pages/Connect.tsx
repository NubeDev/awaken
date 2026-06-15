// Connect screen — establishes the backend connection + credential before any
// tenant-scoped screen renders. This is the desktop/edge story: the same build
// points at any rubix endpoint (ADMIN-UI "Delivery targets"). In browser mode
// the endpoint is left blank (same-origin). The credential is the header pair
// the gate verifies natively; the seed prints subjects like `acme_operator`.

import { useState, type FormEvent } from 'react'
import { ArrowRight } from 'lucide-react'
import { Orb } from '../components/ui/Orb'
import { useConnection } from '../api/ConnectionContext'
import type { Connection } from '../api/connection'

export function Connect() {
  const { connect } = useConnection()
  const [form, setForm] = useState<Connection>({ endpoint: '', subject: '', secret: '', tenant: 'acme' })

  const set = (k: keyof Connection) => (e: React.ChangeEvent<HTMLInputElement>) =>
    setForm((f) => ({ ...f, [k]: e.target.value }))

  const submit = (e: FormEvent) => {
    e.preventDefault()
    if (!form.subject || !form.secret) return
    connect({ ...form, tenant: form.tenant || 'acme' })
  }

  return (
    <div className="h-full grid place-items-center px-6">
      <form onSubmit={submit} className="w-[420px] rounded-2xl border border-border bg-panel2 p-7 shadow-2xl">
        <div className="flex items-center gap-3">
          <Orb size={40} />
          <div>
            <div className="serif text-[22px] font-semibold tracking-tight leading-none">Connect to Rubix</div>
            <div className="text-[12.5px] text-muted mt-1.5">Point at a backend and sign in.</div>
          </div>
        </div>

        <div className="mt-6 space-y-3.5">
          <Field label="Endpoint" hint="blank = same origin">
            <input
              value={form.endpoint}
              onChange={set('endpoint')}
              placeholder="http://127.0.0.1:8088"
              className={inputCls}
              autoComplete="off"
            />
          </Field>
          <Field label="Tenant">
            <input value={form.tenant} onChange={set('tenant')} placeholder="acme" className={inputCls} autoComplete="off" />
          </Field>
          <Field label="Subject">
            <input
              value={form.subject}
              onChange={set('subject')}
              placeholder="acme_operator"
              className={inputCls}
              autoComplete="username"
            />
          </Field>
          <Field label="Secret">
            <input
              value={form.secret}
              onChange={set('secret')}
              type="password"
              placeholder="operator-demo"
              className={inputCls}
              autoComplete="current-password"
            />
          </Field>
        </div>

        <button
          type="submit"
          className="mt-6 w-full h-11 rounded-xl bg-fg text-bg text-[14px] font-semibold flex items-center justify-center gap-2 hover:opacity-90 transition"
        >
          Connect
          <ArrowRight size={16} />
        </button>
        <p className="text-[11.5px] text-muted mt-3 leading-snug">
          The credential is the gate’s native header pair (x-rubix-subject / x-rubix-secret). Run the backend with
          <span className="mono"> SEED=1 </span> to provision the demo cast.
        </p>
      </form>
    </div>
  )
}

const inputCls =
  'w-full h-10 rounded-lg border border-border bg-bg/50 px-3 text-[14px] outline-none placeholder:text-muted focus:border-r1/50 focus:ring-4 focus:ring-r1/10 transition'

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <div className="flex items-center gap-2 mb-1.5">
        <span className="text-[12px] font-medium">{label}</span>
        {hint && <span className="text-[11px] text-muted">· {hint}</span>}
      </div>
      {children}
    </label>
  )
}
