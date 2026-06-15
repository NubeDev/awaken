import { ContentSection } from '../components/content-section'
import { PreferencesForm } from './preferences-form'

export function SettingsPreferences() {
  return (
    <ContentSection
      title='Units & datetime'
      desc='Your preferred units, timezone, and date/time format. The server
          converts values and resolves formatting against these, so every view
          — and a future mobile app — shows the same.'
    >
      <PreferencesForm />
    </ContentSection>
  )
}
