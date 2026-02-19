'use client';

import { useState, useRef, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { apiClient } from '@/lib/api';
import type { ChatMessage, ChatSource, ApiError } from '@/types';
import Link from 'next/link';

interface GeminiChatProps {
  initialMessage?: string;
}

export function GeminiChat({ initialMessage }: GeminiChatProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState(initialMessage || '');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sources, setSources] = useState<ChatSource[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Send initial message if provided
  useEffect(() => {
    if (initialMessage && messages.length === 0) {
      handleSend(initialMessage);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSend = async (messageText?: string) => {
    const textToSend = messageText || input.trim();
    if (!textToSend || isLoading) return;

    const userMessage: ChatMessage = {
      role: 'user',
      content: textToSend,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);
    setError(null);

    try {
      const response = await apiClient.chat({
        message: textToSend,
        history: messages,
      });

      const assistantMessage: ChatMessage = {
        role: 'assistant',
        content: response.answer,
        timestamp: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
      setSources(response.sources || []);
    } catch (err) {
      const apiError = err as ApiError;
      setError(
        apiError.details || apiError.error || 'Ein Fehler ist aufgetreten'
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const getJurisdictionColor = (jurisdiction: string) => {
    switch (jurisdiction) {
      case 'DE':
        return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
      case 'BY':
        return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
      case 'EU':
        return 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
    }
  };

  const getJurisdictionLabel = (jurisdiction: string) => {
    switch (jurisdiction) {
      case 'DE':
        return 'Deutschland';
      case 'BY':
        return 'Bayern';
      case 'EU':
        return 'EU';
      default:
        return jurisdiction;
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-200px)] max-w-5xl mx-auto">
      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto space-y-4 p-4 bg-muted/30 rounded-lg mb-4">
        {messages.length === 0 && (
          <div className="text-center py-12 space-y-4">
            <h2 className="text-2xl font-semibold">Legal AI Assistant</h2>
            <p className="text-muted-foreground max-w-2xl mx-auto">
              Stellen Sie Fragen zu deutschem und EU-Recht. Der Assistent
              durchsucht automatisch relevante Gesetze und Verordnungen.
            </p>
            <div className="flex flex-wrap gap-2 justify-center mt-6">
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  handleSend('Was regelt die DSGVO?')
                }
              >
                Was regelt die DSGVO?
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  handleSend('Erkläre mir den AI Act')
                }
              >
                Erkläre mir den AI Act
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  handleSend(
                    'Welche Rechte habe ich bei Datenschutzverletzungen?'
                  )
                }
              >
                Datenschutzrechte
              </Button>
            </div>
          </div>
        )}

        {messages.map((message, index) => (
          <div
            key={index}
            className={`flex ${
              message.role === 'user' ? 'justify-end' : 'justify-start'
            }`}
          >
            <div
              className={`max-w-[80%] rounded-lg p-4 ${
                message.role === 'user'
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-background border'
              }`}
            >
              <p className="whitespace-pre-wrap">{message.content}</p>
            </div>
          </div>
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="max-w-[80%] rounded-lg p-4 bg-background border">
              <div className="flex items-center space-x-2">
                <div className="h-2 w-2 bg-primary rounded-full animate-pulse" />
                <div className="h-2 w-2 bg-primary rounded-full animate-pulse delay-75" />
                <div className="h-2 w-2 bg-primary rounded-full animate-pulse delay-150" />
              </div>
            </div>
          </div>
        )}

        {error && (
          <div className="flex justify-center">
            <div className="max-w-[80%] rounded-lg p-4 bg-destructive/10 text-destructive border border-destructive/50">
              <p className="font-medium">Fehler</p>
              <p className="text-sm mt-1">{error}</p>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Sources */}
      {sources.length > 0 && (
        <div className="mb-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">
                Quellen ({sources.length})
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {sources.map((source, index) => (
                <div key={index} className="text-sm">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge
                          variant="secondary"
                          className={getJurisdictionColor(source.jurisdiction)}
                        >
                          {getJurisdictionLabel(source.jurisdiction)}
                        </Badge>
                        {source.metadata?.abbreviation && (
                          <span className="font-medium">
                            {source.metadata.abbreviation}
                          </span>
                        )}
                      </div>
                      <p className="font-medium mb-1">{source.title}</p>
                      {source.metadata?.section_reference && (
                        <p className="text-muted-foreground text-xs mb-1">
                          {source.metadata.section_reference}
                        </p>
                      )}
                      <p className="text-muted-foreground line-clamp-2">
                        {source.content}
                      </p>
                    </div>
                    {source.metadata?.source_url && (
                      <Link
                        href={source.metadata.source_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs text-primary hover:underline whitespace-nowrap"
                      >
                        Volltext
                      </Link>
                    )}
                  </div>
                  {index < sources.length - 1 && (
                    <Separator className="mt-3" />
                  )}
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Input Area */}
      <div className="flex gap-2">
        <Input
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder="Stellen Sie eine Frage zu deutschem oder EU-Recht..."
          disabled={isLoading}
          className="flex-1"
        />
        <Button onClick={() => handleSend()} disabled={isLoading || !input.trim()}>
          Senden
        </Button>
      </div>
    </div>
  );
}
