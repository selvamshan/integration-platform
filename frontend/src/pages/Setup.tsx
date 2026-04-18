import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import {
  Check, ChevronRight, ChevronLeft, Copy, ExternalLink,
  Loader2, CheckCircle2, XCircle, Plug, Globe, Shield,
  Key, UserPlus, Rocket, Users,
} from 'lucide-react'
import { useSetupStore, OidcProvider, OidcConfig } from '@/store/setupStore'
// OidcConfig replaces the old KeycloakConfig export
import { authService } from '@/services/auth'

// ── helpers ───────────────────────────────────────────────────────────────────

function CopyBtn({ value }: { value: string }) {
  const [done, setDone] = useState(false)
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(value); setDone(true); setTimeout(() => setDone(false), 2000) }}
      className="ml-1.5 inline-flex items-center gap-0.5 rounded px-1.5 py-0.5 text-xs text-sky-600 hover:bg-sky-50"
    >
      {done ? <Check size={11} /> : <Copy size={11} />}
      {done ? 'Copied' : 'Copy'}
    </button>
  )
}

function Code({ children }: { children: string }) {
  return (
    <span className="inline-flex items-center rounded bg-gray-100 px-2 py-0.5 font-mono text-sm text-gray-800">
      {children}<CopyBtn value={children} />
    </span>
  )
}

function InlineStep({ n, children }: { n: number; children: React.ReactNode }) {
  return (
    <div className="flex gap-3">
      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-sky-100 text-xs font-semibold text-sky-700">{n}</span>
      <div className="text-sm text-gray-700 leading-relaxed">{children}</div>
    </div>
  )
}

function AdminLink({ url }: { url: string }) {
  return (
    <a href={url} target="_blank" rel="noopener noreferrer"
      className="inline-flex items-center gap-1 text-sm text-sky-600 hover:underline break-all">
      {url} <ExternalLink size={12} />
    </a>
  )
}

function Note({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-amber-100 bg-amber-50 p-3 text-xs text-amber-800">{children}</div>
  )
}

function InfoBox({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-sky-100 bg-sky-50 p-4 text-sm">{children}</div>
  )
}

// ── Provider card ─────────────────────────────────────────────────────────────

interface ProviderCardProps {
  id: OidcProvider
  name: string
  description: string
  logo: string
  selected: boolean
  onSelect: () => void
}

function ProviderCard({ id, name, description, logo, selected, onSelect }: ProviderCardProps) {
  return (
    <button
      onClick={onSelect}
      className={`flex flex-col items-center gap-3 rounded-xl border-2 p-6 text-center transition-all
        ${selected
          ? 'border-sky-500 bg-sky-50 shadow-md'
          : 'border-gray-200 bg-white hover:border-sky-300 hover:shadow-sm'}`}
    >
      <span className="text-4xl">{logo}</span>
      <div>
        <p className="font-semibold text-gray-900">{name}</p>
        <p className="mt-0.5 text-xs text-gray-500">{description}</p>
      </div>
      {selected && (
        <span className="inline-flex items-center gap-1 rounded-full bg-sky-600 px-2 py-0.5 text-xs text-white">
          <Check size={10} /> Selected
        </span>
      )}
    </button>
  )
}

// ── Connection forms ──────────────────────────────────────────────────────────

const keycloakSchema = z.object({
  keycloakUrl:     z.string().url('Valid URL required'),
  realm:           z.string().min(1, 'Required'),
  clientId:        z.string().min(1, 'Required'),
  clientSecret:    z.string().min(1, 'Required'),
  controlPlaneUrl: z.string().url('Valid URL required'),
})
type KcForm = z.infer<typeof keycloakSchema>

const auth0Schema = z.object({
  auth0Domain:     z.string().min(3, 'Required (e.g. myapp.us.auth0.com)'),
  clientId:        z.string().min(1, 'Required'),
  clientSecret:    z.string().min(1, 'Required'),
  auth0Audience:   z.string().min(1, 'Required'),
  controlPlaneUrl: z.string().url('Valid URL required'),
})
type Auth0Form = z.infer<typeof auth0Schema>

const oktaSchema = z.object({
  oktaDomain:       z.string().min(3, 'Required (e.g. dev-123.okta.com)'),
  oktaAuthServerId: z.string().min(1, 'Required'),
  clientId:         z.string().min(1, 'Required'),
  clientSecret:     z.string().min(1, 'Required'),
  oktaAudience:     z.string().min(1, 'Required'),
  controlPlaneUrl:  z.string().url('Valid URL required'),
})
type OktaForm = z.infer<typeof oktaSchema>

function Field({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-sm font-medium text-gray-700 mb-1">{label}</label>
      {children}
      {error && <p className="mt-1 text-xs text-red-600">{error}</p>}
    </div>
  )
}

function TestResult({ result, error }: { result: 'ok' | 'fail' | null; error: string }) {
  if (result === 'ok') return <span className="flex items-center gap-1 text-sm text-green-600"><CheckCircle2 size={16} /> Provider reachable</span>
  if (result === 'fail') return <span className="flex items-center gap-1 text-sm text-red-600"><XCircle size={16} /> {error}</span>
  return null
}

// ─── Keycloak connection form ────────────────────────────────────────────────

function KeycloakConnectionForm({ defaults, onSave }: { defaults: Partial<OidcConfig>; onSave: (v: Partial<OidcConfig>) => void }) {
  const [testing, setTesting] = useState(false)
  const [result, setResult] = useState<'ok' | 'fail' | null>(null)
  const [errMsg, setErrMsg] = useState('')

  const { register, handleSubmit, getValues, formState: { errors } } = useForm<KcForm>({
    resolver: zodResolver(keycloakSchema),
    defaultValues: {
      keycloakUrl:     defaults.keycloakUrl || 'http://localhost:8180',
      realm:           defaults.realm || 'integration-platform',
      clientId:        defaults.clientId || 'control-plane',
      clientSecret:    defaults.clientSecret || '',
      controlPlaneUrl: defaults.controlPlaneUrl || 'http://localhost:8081',
    },
  })

  const test = async () => {
    const v = getValues()
    setTesting(true); setResult(null)
    try {
      await authService.testConnection({ ...defaults as OidcConfig, provider: 'keycloak', ...v })
      setResult('ok')
    } catch {
      setResult('fail')
      setErrMsg(`Cannot reach ${v.keycloakUrl}/realms/${v.realm}`)
    } finally { setTesting(false) }
  }

  return (
    <form onSubmit={handleSubmit(onSave)} className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="sm:col-span-2">
          <Field label="Keycloak URL" error={errors.keycloakUrl?.message}>
            <input {...register('keycloakUrl')} placeholder="http://localhost:8180" className="input w-full" />
          </Field>
        </div>
        <Field label="Realm" error={errors.realm?.message}>
          <input {...register('realm')} placeholder="integration-platform" className="input w-full" />
        </Field>
        <Field label="Client ID" error={errors.clientId?.message}>
          <input {...register('clientId')} placeholder="control-plane" className="input w-full" />
        </Field>
        <div className="sm:col-span-2">
          <Field label="Client Secret" error={errors.clientSecret?.message}>
            <input {...register('clientSecret')} type="password" placeholder="••••••••••••••••" className="input w-full" />
          </Field>
        </div>
        <div className="sm:col-span-2">
          <Field label="Control Plane URL" error={errors.controlPlaneUrl?.message}>
            <input {...register('controlPlaneUrl')} placeholder="http://localhost:8081" className="input w-full" />
          </Field>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <button type="button" onClick={test} disabled={testing} className="btn btn-secondary flex items-center gap-2">
          {testing ? <Loader2 size={14} className="animate-spin" /> : <Plug size={14} />} Test Connection
        </button>
        <TestResult result={result} error={errMsg} />
      </div>
      <button type="submit" className="btn btn-primary">Save & Continue <ChevronRight size={16} className="inline ml-1" /></button>
    </form>
  )
}

// ─── Auth0 connection form ───────────────────────────────────────────────────

function Auth0ConnectionForm({ defaults, onSave }: { defaults: Partial<OidcConfig>; onSave: (v: Partial<OidcConfig>) => void }) {
  const [testing, setTesting] = useState(false)
  const [result, setResult] = useState<'ok' | 'fail' | null>(null)
  const [errMsg, setErrMsg] = useState('')

  const { register, handleSubmit, getValues, formState: { errors } } = useForm<Auth0Form>({
    resolver: zodResolver(auth0Schema),
    defaultValues: {
      auth0Domain:     defaults.auth0Domain || '',
      clientId:        defaults.clientId || '',
      clientSecret:    defaults.clientSecret || '',
      auth0Audience:   defaults.auth0Audience || '',
      controlPlaneUrl: defaults.controlPlaneUrl || 'http://localhost:8081',
    },
  })

  const test = async () => {
    const v = getValues()
    setTesting(true); setResult(null)
    try {
      await authService.testConnection({ ...defaults as OidcConfig, provider: 'auth0', ...v })
      setResult('ok')
    } catch {
      setResult('fail')
      setErrMsg(`Cannot reach https://${v.auth0Domain}/.well-known/openid-configuration`)
    } finally { setTesting(false) }
  }

  return (
    <form onSubmit={handleSubmit(onSave)} className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="sm:col-span-2">
          <Field label="Auth0 Domain" error={errors.auth0Domain?.message}>
            <input {...register('auth0Domain')} placeholder="your-tenant.us.auth0.com" className="input w-full" />
          </Field>
        </div>
        <Field label="Client ID" error={errors.clientId?.message}>
          <input {...register('clientId')} placeholder="From Auth0 application" className="input w-full" />
        </Field>
        <Field label="Client Secret" error={errors.clientSecret?.message}>
          <input {...register('clientSecret')} type="password" placeholder="••••••••••••" className="input w-full" />
        </Field>
        <div className="sm:col-span-2">
          <Field label="API Audience (identifier)" error={errors.auth0Audience?.message}>
            <input {...register('auth0Audience')} placeholder="https://api.your-app.com" className="input w-full" />
          </Field>
        </div>
        <div className="sm:col-span-2">
          <Field label="Control Plane URL" error={errors.controlPlaneUrl?.message}>
            <input {...register('controlPlaneUrl')} placeholder="http://localhost:8081" className="input w-full" />
          </Field>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <button type="button" onClick={test} disabled={testing} className="btn btn-secondary flex items-center gap-2">
          {testing ? <Loader2 size={14} className="animate-spin" /> : <Plug size={14} />} Test Connection
        </button>
        <TestResult result={result} error={errMsg} />
      </div>
      <button type="submit" className="btn btn-primary">Save & Continue <ChevronRight size={16} className="inline ml-1" /></button>
    </form>
  )
}

// ─── Okta connection form ────────────────────────────────────────────────────

function OktaConnectionForm({ defaults, onSave }: { defaults: Partial<OidcConfig>; onSave: (v: Partial<OidcConfig>) => void }) {
  const [testing, setTesting] = useState(false)
  const [result, setResult] = useState<'ok' | 'fail' | null>(null)
  const [errMsg, setErrMsg] = useState('')

  const { register, handleSubmit, getValues, formState: { errors } } = useForm<OktaForm>({
    resolver: zodResolver(oktaSchema),
    defaultValues: {
      oktaDomain:       defaults.oktaDomain || '',
      oktaAuthServerId: defaults.oktaAuthServerId || 'default',
      clientId:         defaults.clientId || '',
      clientSecret:     defaults.clientSecret || '',
      oktaAudience:     defaults.oktaAudience || 'api://default',
      controlPlaneUrl:  defaults.controlPlaneUrl || 'http://localhost:8081',
    },
  })

  const test = async () => {
    const v = getValues()
    setTesting(true); setResult(null)
    try {
      await authService.testConnection({ ...defaults as OidcConfig, provider: 'okta', ...v })
      setResult('ok')
    } catch {
      setResult('fail')
      setErrMsg(`Cannot reach https://${v.oktaDomain}/oauth2/${v.oktaAuthServerId}`)
    } finally { setTesting(false) }
  }

  return (
    <form onSubmit={handleSubmit(onSave)} className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="sm:col-span-2">
          <Field label="Okta Domain" error={errors.oktaDomain?.message}>
            <input {...register('oktaDomain')} placeholder="dev-123456.okta.com" className="input w-full" />
          </Field>
        </div>
        <Field label="Authorization Server ID" error={errors.oktaAuthServerId?.message}>
          <input {...register('oktaAuthServerId')} placeholder="default" className="input w-full" />
        </Field>
        <Field label="Audience" error={errors.oktaAudience?.message}>
          <input {...register('oktaAudience')} placeholder="api://default" className="input w-full" />
        </Field>
        <Field label="Client ID" error={errors.clientId?.message}>
          <input {...register('clientId')} placeholder="From Okta application" className="input w-full" />
        </Field>
        <Field label="Client Secret" error={errors.clientSecret?.message}>
          <input {...register('clientSecret')} type="password" placeholder="••••••••••••" className="input w-full" />
        </Field>
        <div className="sm:col-span-2">
          <Field label="Control Plane URL" error={errors.controlPlaneUrl?.message}>
            <input {...register('controlPlaneUrl')} placeholder="http://localhost:8081" className="input w-full" />
          </Field>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <button type="button" onClick={test} disabled={testing} className="btn btn-secondary flex items-center gap-2">
          {testing ? <Loader2 size={14} className="animate-spin" /> : <Plug size={14} />} Test Connection
        </button>
        <TestResult result={result} error={errMsg} />
      </div>
      <button type="submit" className="btn btn-primary">Save & Continue <ChevronRight size={16} className="inline ml-1" /></button>
    </form>
  )
}

// ── Provider-specific guide steps ─────────────────────────────────────────────

function KeycloakRealmGuide({ cfg }: { cfg: OidcConfig }) {
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Keycloak Admin Console</p>
        <AdminLink url={`${cfg.keycloakUrl}/admin`} />
        <p className="mt-1 text-xs text-sky-700">Default credentials: <strong>admin / admin</strong></p>
      </InfoBox>
      <InlineStep n={1}>Open Admin Console and sign in.</InlineStep>
      <InlineStep n={2}>Click the realm dropdown (shows <strong>master</strong>) → <strong>Create Realm</strong>.</InlineStep>
      <InlineStep n={3}>Set Realm name: <Code>{cfg.realm}</Code></InlineStep>
      <InlineStep n={4}>Toggle <strong>Enabled</strong> ON → <strong>Create</strong>.</InlineStep>
    </div>
  )
}

function KeycloakClientGuide({ cfg }: { cfg: OidcConfig }) {
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Navigate to Clients</p>
        <AdminLink url={`${cfg.keycloakUrl}/admin/master/console/#/${cfg.realm}/clients`} />
      </InfoBox>
      <InlineStep n={1}>Clients → <strong>Create client</strong> → set Client type: <strong>OpenID Connect</strong>.</InlineStep>
      <InlineStep n={2}>Client ID: <Code>{cfg.clientId}</Code> → <strong>Next</strong>.</InlineStep>
      <InlineStep n={3}>Enable <strong>Client authentication</strong> (confidential) and <strong>Direct access grants</strong> → <strong>Next</strong> → <strong>Save</strong>.</InlineStep>
      <InlineStep n={4}>Open the <strong>Credentials</strong> tab → copy the Client Secret and paste it back in the Connection step.</InlineStep>
      <InlineStep n={5}>Open <strong>Settings</strong> → Valid redirect URIs: add <Code>http://localhost:3000/*</Code></InlineStep>
    </div>
  )
}

function KeycloakRolesGuide({ cfg }: { cfg: OidcConfig }) {
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Realm Roles</p>
        <AdminLink url={`${cfg.keycloakUrl}/admin/master/console/#/${cfg.realm}/roles`} />
      </InfoBox>
      <InlineStep n={1}>Realm roles → <strong>Create role</strong> → name: <Code>admin</Code> → Save.</InlineStep>
      <InlineStep n={2}>Repeat for <Code>developer</Code> and <Code>viewer</Code>.</InlineStep>
      <Note>The backend maps these exact role names to RBAC permissions.</Note>
    </div>
  )
}

function KeycloakUserGuide({ cfg }: { cfg: OidcConfig }) {
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Users</p>
        <AdminLink url={`${cfg.keycloakUrl}/admin/master/console/#/${cfg.realm}/users`} />
      </InfoBox>
      <InlineStep n={1}>Users → <strong>Create new user</strong> → set Username & Email → <strong>Create</strong>.</InlineStep>
      <InlineStep n={2}><strong>Credentials</strong> tab → <strong>Set password</strong> → disable <em>Temporary</em> → Save.</InlineStep>
      <InlineStep n={3}><strong>Role mapping</strong> tab → <strong>Assign role</strong> → select <Code>admin</Code> → Assign.</InlineStep>
    </div>
  )
}

// ─── Auth0 guides ────────────────────────────────────────────────────────────

function Auth0AppGuide({ cfg }: { cfg: OidcConfig }) {
  const dashUrl = `https://manage.auth0.com/dashboard`
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Auth0 Dashboard</p>
        <AdminLink url={dashUrl} />
      </InfoBox>
      <InlineStep n={1}>Applications → <strong>Create Application</strong> → choose <strong>Regular Web Application</strong> → Create.</InlineStep>
      <InlineStep n={2}>Copy <strong>Client ID</strong> and <strong>Client Secret</strong> from the Settings tab — paste them in the Connection step.</InlineStep>
      <InlineStep n={3}>Under <strong>Allowed Callback URLs</strong> add <Code>http://localhost:3000</Code> → Save.</InlineStep>
      <InlineStep n={4}>Enable <strong>Password</strong> grant: Settings → Advanced → Grant Types → check <em>Password</em> → Save.</InlineStep>
      <InlineStep n={5}>APIs → <strong>Create API</strong> → set Identifier to your audience value (e.g. <Code>https://api.your-app.com</Code>).</InlineStep>
      <Note>Auth0 Resource Owner Password Grant must be enabled explicitly. It is only available on the <strong>Default Directory</strong> connection.</Note>
    </div>
  )
}

function Auth0RolesGuide({ cfg }: { cfg: OidcConfig }) {
  const ns = 'https://integration-platform/roles'
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Auth0 RBAC Setup</p>
        <AdminLink url="https://manage.auth0.com/#/roles" />
      </InfoBox>
      <InlineStep n={1}>User Management → <strong>Roles</strong> → <strong>Create Role</strong>.</InlineStep>
      <InlineStep n={2}>Create roles: <Code>admin</Code>, <Code>developer</Code>, <Code>viewer</Code>.</InlineStep>
      <InlineStep n={3}>Go to <strong>Actions</strong> → Library → <strong>Build Custom</strong> → trigger: <em>Login / Post Login</em>.</InlineStep>
      <InlineStep n={4}>Paste this Action code to inject roles into the token:</InlineStep>
      <pre className="rounded-lg bg-gray-900 text-gray-100 text-xs p-4 overflow-x-auto whitespace-pre">{`exports.onExecutePostLogin = async (event, api) => {
  const namespace = '${ns}';
  const roles = event.authorization?.roles ?? [];
  api.idToken.setCustomClaim(namespace, roles);
  api.accessToken.setCustomClaim(namespace, roles);
};`}</pre>
      <InlineStep n={5}>Deploy the Action and add it to the <strong>Post Login</strong> flow.</InlineStep>
      <Note>The backend reads roles from the <Code>{ns}</Code> claim namespace.</Note>
    </div>
  )
}

function Auth0UserGuide() {
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Create First User</p>
        <AdminLink url="https://manage.auth0.com/#/users" />
      </InfoBox>
      <InlineStep n={1}>User Management → Users → <strong>Create User</strong> → enter Email and Password → Create.</InlineStep>
      <InlineStep n={2}>Open the user → <strong>Roles</strong> tab → <strong>Assign Roles</strong> → select <Code>admin</Code>.</InlineStep>
    </div>
  )
}

// ─── Okta guides ─────────────────────────────────────────────────────────────

function OktaAppGuide({ cfg }: { cfg: OidcConfig }) {
  const adminUrl = `https://${cfg.oktaDomain || 'your-domain.okta.com'}/admin`
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Okta Admin Console</p>
        <AdminLink url={adminUrl} />
      </InfoBox>
      <InlineStep n={1}>Applications → <strong>Create App Integration</strong> → <strong>OIDC — OpenID Connect</strong> → <strong>Web Application</strong> → Next.</InlineStep>
      <InlineStep n={2}>Set Sign-in redirect URI: <Code>http://localhost:3000</Code> → Save.</InlineStep>
      <InlineStep n={3}>Copy <strong>Client ID</strong> and <strong>Client secret</strong> — paste them in the Connection step.</InlineStep>
      <InlineStep n={4}>Assignments → assign to <em>Everyone</em> or a specific group.</InlineStep>
      <InlineStep n={5}>Security → API → <strong>Authorization Servers</strong> → edit <em>default</em> → enable <strong>Resource Owner Password</strong> grant.</InlineStep>
      <Note>Okta ROPC requires the Password grant to be enabled on the Authorization Server.</Note>
    </div>
  )
}

function OktaGroupsGuide({ cfg }: { cfg: OidcConfig }) {
  const adminUrl = `https://${cfg.oktaDomain || 'your-domain.okta.com'}/admin/groups`
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Okta Groups (used as roles)</p>
        <AdminLink url={adminUrl} />
      </InfoBox>
      <InlineStep n={1}>Directory → Groups → <strong>Add group</strong> → name: <Code>admin</Code> → Save.</InlineStep>
      <InlineStep n={2}>Repeat for <Code>developer</Code> and <Code>viewer</Code>.</InlineStep>
      <InlineStep n={3}>Security → API → Authorization Servers → <em>default</em> → Claims tab.</InlineStep>
      <InlineStep n={4}>Add Claim: Name <Code>groups</Code>, Include in token type <em>Access Token</em>, Value type <em>Groups</em>, Filter <em>Matches regex</em> <Code>.*</Code>.</InlineStep>
      <Note>The backend reads group names from the <Code>groups</Code> claim and maps them to <Code>admin</Code> / <Code>developer</Code> / <Code>viewer</Code>.</Note>
    </div>
  )
}

function OktaUserGuide({ cfg }: { cfg: OidcConfig }) {
  const adminUrl = `https://${cfg.oktaDomain || 'your-domain.okta.com'}/admin/users`
  return (
    <div className="space-y-3">
      <InfoBox>
        <p className="font-medium text-sky-800 mb-1">Create First User</p>
        <AdminLink url={adminUrl} />
      </InfoBox>
      <InlineStep n={1}>Directory → People → <strong>Add person</strong> → fill in details → set password → Save.</InlineStep>
      <InlineStep n={2}>Open the user → <strong>Groups</strong> tab → <strong>Add to group</strong> → select <Code>admin</Code>.</InlineStep>
    </div>
  )
}

// ── Step navigation ───────────────────────────────────────────────────────────

function StepNav({ onBack, onNext, nextLabel = 'Next' }: { onBack: () => void; onNext: () => void; nextLabel?: string }) {
  return (
    <div className="flex gap-3 pt-2 border-t border-gray-100">
      <button onClick={onBack} className="btn btn-secondary flex items-center gap-1">
        <ChevronLeft size={14} /> Back
      </button>
      <button onClick={onNext} className="btn btn-primary flex items-center gap-1">
        {nextLabel} <ChevronRight size={14} />
      </button>
    </div>
  )
}

// ── Wizard definition per provider ───────────────────────────────────────────

type StepId = string

interface WizardStep {
  id: StepId
  label: string
  icon: React.ElementType
}

function getSteps(provider: OidcProvider): WizardStep[] {
  const base: WizardStep[] = [
    { id: 'welcome',    label: 'Welcome',    icon: Rocket },
    { id: 'provider',   label: 'Provider',   icon: Users },
    { id: 'connection', label: 'Connection', icon: Plug },
  ]
  const done: WizardStep = { id: 'done', label: 'Done', icon: CheckCircle2 }

  if (provider === 'auth0') {
    return [...base,
      { id: 'app',    label: 'Application', icon: Globe },
      { id: 'roles',  label: 'Roles',       icon: Key },
      { id: 'user',   label: 'Admin User',  icon: UserPlus },
      done,
    ]
  }
  if (provider === 'okta') {
    return [...base,
      { id: 'app',    label: 'Application', icon: Globe },
      { id: 'groups', label: 'Groups',      icon: Key },
      { id: 'user',   label: 'Admin User',  icon: UserPlus },
      done,
    ]
  }
  // keycloak
  return [...base,
    { id: 'realm',  label: 'Realm',      icon: Globe },
    { id: 'client', label: 'Client',     icon: Shield },
    { id: 'roles',  label: 'Roles',      icon: Key },
    { id: 'user',   label: 'Admin User', icon: UserPlus },
    done,
  ]
}

// ── Main wizard ───────────────────────────────────────────────────────────────

export function Setup() {
  const navigate = useNavigate()
  const store = useSetupStore()
  const [stepIndex, setStepIndex] = useState(0)

  const cfg: OidcConfig = {
    provider:        store.provider,
    controlPlaneUrl: store.controlPlaneUrl,
    keycloakUrl:     store.keycloakUrl,
    realm:           store.realm,
    clientId:        store.clientId,
    clientSecret:    store.clientSecret,
    auth0Domain:     store.auth0Domain,
    auth0Audience:   store.auth0Audience,
    oktaDomain:      store.oktaDomain,
    oktaAuthServerId: store.oktaAuthServerId,
    oktaAudience:    store.oktaAudience,
  }

  const steps = getSteps(store.provider)
  const currentStep = steps[stepIndex]

  const next = () => setStepIndex((i) => Math.min(i + 1, steps.length - 1))
  const back = () => setStepIndex((i) => Math.max(i - 1, 0))

  const saveConfig = (partial: Partial<OidcConfig>) => { store.setConfig(partial); next() }

  const finish = () => { store.markConfigured(); navigate('/login') }
  const skip   = () => { store.markConfigured(); navigate('/login') }

  const providerLabel = { keycloak: 'Keycloak', auth0: 'Auth0', okta: 'Okta' }[store.provider]

  return (
    <div className="min-h-screen bg-gray-50 flex flex-col items-center justify-center px-4 py-12">
      {/* Header */}
      <div className="mb-8 text-center">
        <div className="flex items-center justify-center gap-2 mb-2">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-sky-600">
            <Plug size={20} className="text-white" />
          </div>
          <span className="text-xl font-bold text-gray-900">Integration Platform</span>
        </div>
        <p className="text-sm text-gray-500">Initial Setup</p>
      </div>

      {/* Progress */}
      <div className="w-full max-w-2xl mb-6">
        <div className="flex items-end justify-between">
          {steps.map((s, i) => {
            const Icon = s.icon
            const done   = i < stepIndex
            const active = i === stepIndex
            return (
              <div key={s.id} className="flex flex-1 flex-col items-center">
                <div className={`flex h-8 w-8 items-center justify-center rounded-full border-2 transition-colors
                  ${done ? 'border-sky-600 bg-sky-600 text-white' : active ? 'border-sky-600 bg-white text-sky-600' : 'border-gray-300 bg-white text-gray-400'}`}>
                  {done ? <Check size={14} /> : <Icon size={14} />}
                </div>
                <span className={`mt-1 hidden text-xs sm:block ${active ? 'font-semibold text-sky-600' : 'text-gray-400'}`}>{s.label}</span>
              </div>
            )
          })}
        </div>
        <div className="mt-3 flex">
          {steps.slice(0, -1).map((_, i) => (
            <div key={i} className={`flex-1 h-0.5 ${i < stepIndex ? 'bg-sky-600' : 'bg-gray-200'}`} />
          ))}
        </div>
      </div>

      {/* Card */}
      <div className="w-full max-w-2xl rounded-2xl bg-white shadow-sm border border-gray-200 overflow-hidden">
        <div className="border-b border-gray-100 px-8 py-5">
          <h2 className="text-lg font-semibold text-gray-900">
            {currentStep.id === 'welcome'    && 'Welcome to Integration Platform'}
            {currentStep.id === 'provider'   && 'Choose Your OIDC Provider'}
            {currentStep.id === 'connection' && `${providerLabel} Connection Settings`}
            {currentStep.id === 'realm'      && 'Step 1 — Create Keycloak Realm'}
            {currentStep.id === 'client'     && 'Step 2 — Create OIDC Client'}
            {currentStep.id === 'roles'      && `Step ${store.provider === 'keycloak' ? 3 : 2} — Configure Roles`}
            {currentStep.id === 'user'       && 'Step — Create Admin User'}
            {currentStep.id === 'app'        && `Step 1 — Create ${providerLabel} Application`}
            {currentStep.id === 'groups'     && 'Step 2 — Configure Okta Groups'}
            {currentStep.id === 'done'       && 'Setup Complete!'}
          </h2>
          <p className="text-xs text-gray-400 mt-0.5">Step {stepIndex + 1} of {steps.length}</p>
        </div>

        <div className="px-8 py-6">
          {/* Welcome */}
          {currentStep.id === 'welcome' && (
            <div className="space-y-5">
              <p className="text-sm text-gray-600 leading-relaxed">
                This wizard guides you through connecting Integration Platform to your identity provider.
                Choose one of the supported OIDC providers:
              </p>
              <ul className="space-y-2 text-sm text-gray-700">
                {['🔑  Keycloak — self-hosted, full control', '🔐  Auth0 — cloud-hosted, developer-friendly', '🛡️  Okta — enterprise identity platform'].map((item, i) => (
                  <li key={i} className="flex items-start gap-2"><CheckCircle2 size={16} className="mt-0.5 text-sky-500 shrink-0" />{item}</li>
                ))}
              </ul>
              <div className="flex gap-3">
                <button onClick={next} className="btn btn-primary">Get Started <ChevronRight size={16} className="inline ml-1" /></button>
                <button onClick={skip} className="btn btn-secondary text-xs">Skip — already configured</button>
              </div>
            </div>
          )}

          {/* Provider selection */}
          {currentStep.id === 'provider' && (
            <div className="space-y-6">
              <div className="grid grid-cols-3 gap-4">
                <ProviderCard id="keycloak" name="Keycloak" description="Self-hosted OIDC" logo="🔑"
                  selected={store.provider === 'keycloak'} onSelect={() => store.setProvider('keycloak')} />
                <ProviderCard id="auth0" name="Auth0" description="Cloud identity" logo="🔐"
                  selected={store.provider === 'auth0'} onSelect={() => store.setProvider('auth0')} />
                <ProviderCard id="okta" name="Okta" description="Enterprise identity" logo="🛡️"
                  selected={store.provider === 'okta'} onSelect={() => store.setProvider('okta')} />
              </div>
              <div className="flex gap-3 pt-2 border-t border-gray-100">
                <button onClick={back} className="btn btn-secondary flex items-center gap-1"><ChevronLeft size={14} /> Back</button>
                <button onClick={next} className="btn btn-primary flex items-center gap-1">Use {providerLabel} <ChevronRight size={14} /></button>
              </div>
            </div>
          )}

          {/* Connection config — per provider */}
          {currentStep.id === 'connection' && store.provider === 'keycloak' && (
            <KeycloakConnectionForm defaults={cfg} onSave={saveConfig} />
          )}
          {currentStep.id === 'connection' && store.provider === 'auth0' && (
            <Auth0ConnectionForm defaults={cfg} onSave={saveConfig} />
          )}
          {currentStep.id === 'connection' && store.provider === 'okta' && (
            <OktaConnectionForm defaults={cfg} onSave={saveConfig} />
          )}

          {/* Keycloak steps */}
          {currentStep.id === 'realm'  && <div className="space-y-5"><KeycloakRealmGuide  cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'client' && <div className="space-y-5"><KeycloakClientGuide cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'roles'  && store.provider === 'keycloak' && <div className="space-y-5"><KeycloakRolesGuide cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'user'   && store.provider === 'keycloak' && <div className="space-y-5"><KeycloakUserGuide  cfg={cfg} /><StepNav onBack={back} onNext={next} nextLabel="Next: Finish" /></div>}

          {/* Auth0 steps */}
          {currentStep.id === 'app'   && store.provider === 'auth0' && <div className="space-y-5"><Auth0AppGuide   cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'roles' && store.provider === 'auth0' && <div className="space-y-5"><Auth0RolesGuide cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'user'  && store.provider === 'auth0' && <div className="space-y-5"><Auth0UserGuide              /><StepNav onBack={back} onNext={next} nextLabel="Next: Finish" /></div>}

          {/* Okta steps */}
          {currentStep.id === 'app'    && store.provider === 'okta' && <div className="space-y-5"><OktaAppGuide    cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'groups' && store.provider === 'okta' && <div className="space-y-5"><OktaGroupsGuide cfg={cfg} /><StepNav onBack={back} onNext={next} /></div>}
          {currentStep.id === 'user'   && store.provider === 'okta' && <div className="space-y-5"><OktaUserGuide   cfg={cfg} /><StepNav onBack={back} onNext={next} nextLabel="Next: Finish" /></div>}

          {/* Done */}
          {currentStep.id === 'done' && (
            <div className="space-y-5 text-center">
              <div className="flex justify-center"><CheckCircle2 size={56} className="text-green-500" /></div>
              <div>
                <p className="text-base font-medium text-gray-800">{providerLabel} is configured and ready.</p>
                <p className="text-sm text-gray-500 mt-1">Log in with the admin user you just created.</p>
              </div>
              <div className="rounded-lg border border-gray-100 bg-gray-50 p-4 text-left text-sm">
                <p className="font-medium text-gray-700 mb-2">Configuration summary</p>
                <dl className="space-y-1 text-xs text-gray-600">
                  <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Provider</dt><dd className="font-mono">{providerLabel}</dd></div>
                  {store.provider === 'keycloak' && <>
                    <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Keycloak URL</dt><dd className="font-mono truncate">{cfg.keycloakUrl}</dd></div>
                    <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Realm</dt><dd className="font-mono">{cfg.realm}</dd></div>
                  </>}
                  {store.provider === 'auth0' && (
                    <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Domain</dt><dd className="font-mono">{cfg.auth0Domain}</dd></div>
                  )}
                  {store.provider === 'okta' && (
                    <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Domain</dt><dd className="font-mono">{cfg.oktaDomain}</dd></div>
                  )}
                  <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Client ID</dt><dd className="font-mono">{cfg.clientId}</dd></div>
                  <div className="flex gap-2"><dt className="w-28 font-medium shrink-0">Control Plane</dt><dd className="font-mono truncate">{cfg.controlPlaneUrl}</dd></div>
                </dl>
              </div>
              <div className="flex justify-center gap-3">
                <button onClick={back} className="btn btn-secondary flex items-center gap-1"><ChevronLeft size={14} /> Back</button>
                <button onClick={finish} className="btn btn-primary">Go to Login <ChevronRight size={16} className="inline ml-1" /></button>
              </div>
            </div>
          )}
        </div>
      </div>

      <p className="mt-6 text-xs text-gray-400">
        Config is stored in your browser. Reconfigure any time via the Admin menu.
      </p>
    </div>
  )
}
