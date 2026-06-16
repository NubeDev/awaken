import { useState } from 'react'
import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useNavigate } from '@tanstack/react-router'
import { LogIn } from 'lucide-react'
import { toast } from 'sonner'
import { ApiError } from '@/api/client'
import {
  login,
  fetchMe,
  DEMO_PRINCIPALS,
  type DemoPrincipal,
} from '@/api/auth'
import { useAuthStore } from '@/stores/auth-store'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { PasswordInput } from '@/components/password-input'

const formSchema = z.object({
  subject: z.string().min(1, 'Enter your username.'),
  secret: z.string().min(1, 'Enter your password.'),
})

interface UserAuthFormProps extends React.HTMLAttributes<HTMLFormElement> {
  redirectTo?: string
}

export function UserAuthForm({
  className,
  redirectTo,
  ...props
}: UserAuthFormProps) {
  const [isLoading, setIsLoading] = useState(false)
  const navigate = useNavigate()
  const { auth } = useAuthStore()

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: { subject: '', secret: '' },
  })

  /**
   * Exchange the credentials for a login token, reflect the identity, persist
   * the session, then land on the redirect. Shared by the form submit and the
   * one-click demo buttons.
   */
  async function signIn(subject: string, secret: string) {
    setIsLoading(true)
    try {
      const { token } = await login({ subject, secret })
      // Reflect who we are so the chrome can show the principal + gate admin UI.
      // A failure here is non-fatal — the token is still valid.
      const me = await fetchMe(token).catch(() => null)
      auth.setSession(
        token,
        me
          ? { subject: me.subject, namespace: me.namespace, role: me.role }
          : null
      )
      toast.success(`Signed in${me ? ` as ${me.subject}` : ''}`)
      navigate({ to: redirectTo || '/', replace: true })
    } catch (err) {
      const msg =
        err instanceof ApiError && err.status === 401
          ? 'Invalid username or password.'
          : err instanceof Error
            ? err.message
            : 'Sign-in failed.'
      toast.error(msg)
    } finally {
      setIsLoading(false)
    }
  }

  function onSubmit(data: z.infer<typeof formSchema>) {
    void signIn(data.subject.trim(), data.secret)
  }

  function onDemo(p: DemoPrincipal) {
    void signIn(p.subject, p.secret)
  }

  return (
    <div className={cn('grid gap-6', className)}>
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)} className='grid gap-3' {...props}>
          <FormField
            control={form.control}
            name='subject'
            render={({ field }) => (
              <FormItem>
                <FormLabel>Username</FormLabel>
                <FormControl>
                  <Input placeholder='acme_admin' autoComplete='username' {...field} />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name='secret'
            render={({ field }) => (
              <FormItem>
                <FormLabel>Password</FormLabel>
                <FormControl>
                  <PasswordInput
                    placeholder='••••••••'
                    autoComplete='current-password'
                    {...field}
                  />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
          <Button className='mt-2' disabled={isLoading}>
            <LogIn />
            Sign in
          </Button>
        </form>
      </Form>

      <div className='relative'>
        <div className='absolute inset-0 flex items-center'>
          <span className='w-full border-t' />
        </div>
        <div className='relative flex justify-center text-xs uppercase'>
          <span className='bg-card text-muted-foreground px-2'>
            Demo sign-in
          </span>
        </div>
      </div>

      <div className='grid gap-2'>
        {DEMO_PRINCIPALS.map((p) => (
          <Button
            key={p.subject}
            type='button'
            variant='outline'
            disabled={isLoading}
            onClick={() => onDemo(p)}
            className='h-auto justify-start py-2 text-start'
          >
            <div className='grid gap-0.5'>
              <span className='font-medium'>{p.label}</span>
              <span className='text-muted-foreground text-xs font-normal'>
                {p.blurb}
              </span>
            </div>
          </Button>
        ))}
      </div>
    </div>
  )
}
