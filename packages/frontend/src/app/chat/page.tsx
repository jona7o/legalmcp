import { GeminiChat } from '@/components/chat/GeminiChat';

export default function ChatPage() {
  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">
          Legal AI Assistent
        </h1>
        <p className="text-muted-foreground max-w-2xl mx-auto">
          Powered by Gemini 2.5 Pro mit Retrieval Augmented Generation über
          deutsche und EU-Rechtsquellen
        </p>
      </div>

      <GeminiChat />
    </div>
  );
}
