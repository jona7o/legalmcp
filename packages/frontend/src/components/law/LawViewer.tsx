'use client';

import ReactMarkdown from 'react-markdown';
import { ExternalLink, Calendar, Sparkles } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import type { Document } from '@/types';

interface LawViewerProps {
  document: Document;
  t: (key: string) => string;
}

export function LawViewer({ document, t }: LawViewerProps) {
  return (
    <div className="space-y-6">
      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between gap-4">
            <div>
              <CardTitle className="text-3xl">{document.title}</CardTitle>
              {document.source_name && (
                <p className="mt-2 text-lg text-muted-foreground">
                  {document.source_name}
                </p>
              )}
            </div>
            <Badge variant="outline" className="text-lg px-4 py-2">
              {document.jurisdiction}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
            {document.published_at && (
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4" />
                <span>
                  {t('publishedAt')}:{' '}
                  {new Date(document.published_at).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>

          {document.url && (
            <a
              href={document.url}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-sm text-primary hover:underline"
            >
              <ExternalLink className="h-4 w-4" />
              {t('originalSource')}
            </a>
          )}
        </CardContent>
      </Card>

      {/* AI Summary */}
      {document.summary && (
        <Card className="border-primary/30 bg-primary/5">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Sparkles className="h-4 w-4 text-primary" />
              {t('summary')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm leading-relaxed">{document.summary}</p>
          </CardContent>
        </Card>
      )}

      {/* Full Text */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">{t('fullText')}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="prose prose-sm max-w-none dark:prose-invert">
            <ReactMarkdown>{document.content_md}</ReactMarkdown>
          </div>
        </CardContent>
      </Card>

      {/* Disclaimer */}
      <Card className="border-yellow-500/50 bg-yellow-50 dark:bg-yellow-950/20">
        <CardContent className="pt-6">
          <p className="text-sm text-muted-foreground">{t('disclaimer')}</p>
        </CardContent>
      </Card>
    </div>
  );
}
