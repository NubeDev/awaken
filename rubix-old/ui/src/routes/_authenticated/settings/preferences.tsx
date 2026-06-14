import { createFileRoute } from '@tanstack/react-router'
import { SettingsPreferences } from '@/features/settings/preferences'

export const Route = createFileRoute('/_authenticated/settings/preferences')({
  component: SettingsPreferences,
})
