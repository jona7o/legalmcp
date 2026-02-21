import { useTranslations } from 'next-intl';
import { GeminiChat } from '@/components/chat/GeminiChat';

// Chat endpoint is deferred — no new Rust API endpoint yet.
// This page is kept as-is, just moved to the [locale] routing.

export default function ChatPage() {
  // Server component — use getTranslations if needed, or keep static strings
  // for now since the endpoint is deferred.
  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">Legal AI Assistent</h1>
        <p className="text-muted-foreground max-w-2xl mx-auto">
          Powered by Gemini 2.5 Pro mit Retrieval Augmented Generation über
          deutsche und EU-Rechtsquellen
        </p>
      </div>

      <GeminiChat />
    </div>
  );
}
