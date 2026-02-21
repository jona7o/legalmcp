'use client';

import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { FileText, Gavel, BookOpen, Scale } from 'lucide-react';
import type { SearchResult, DocType } from '@/types';

interface ResultListProps {
  results: SearchResult[];
  isLoading?: boolean;
  error?: string | null;
}

const DOC_TYPE_ICONS: Record<DocType, React.ReactNode> = {
  statute: <FileText className="mt-1 h-5 w-5 text-primary" />,
  regulation: <BookOpen className="mt-1 h-5 w-5 text-primary" />,
  case: <Gavel className="mt-1 h-5 w-5 text-primary" />,
  directive: <Scale className="mt-1 h-5 w-5 text-primary" />,
};

export function ResultList({ results, isLoading, error }: ResultListProps) {
  const t = useTranslations('result');
  const tSearch = useTranslations('search');

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[...Array(5)].map((_, i) => (
          <Card key={i} className="animate-pulse">
            <CardHeader>
              <div className="h-6 w-3/4 rounded bg-muted" />
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="h-4 w-full rounded bg-muted" />
                <div className="h-4 w-5/6 rounded bg-muted" />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <Card className="border-destructive">
        <CardContent className="pt-6">
          <p className="text-sm text-destructive">
            <strong>{tSearch('error')}:</strong> {error}
          </p>
        </CardContent>
      </Card>
    );
  }

  if (results.length === 0) {
    return (
      <Card>
        <CardContent className="pt-6">
          <p className="text-center text-muted-foreground">{tSearch('noResults')}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {results.map((result) => (
        <Link key={result.chunk_id} href={`/law/${result.document_id}`}>
          <Card className="transition-shadow hover:shadow-md">
            <CardHeader>
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-start gap-3 flex-1">
                  {DOC_TYPE_ICONS[result.doc_type] ?? (
                    <FileText className="mt-1 h-5 w-5 text-primary" />
                  )}
                  <div className="flex-1">
                    <h3 className="font-semibold leading-tight">{result.title}</h3>
                    {result.source_name && (
                      <p className="mt-1 text-sm text-muted-foreground">
                        {t('source')}: {result.source_name}
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex flex-col items-end gap-2">
                  <div className="flex gap-2">
                    <Badge variant="outline">{result.jurisdiction}</Badge>
                    <Badge variant="secondary">
                      {t(result.doc_type)}
                    </Badge>
                  </div>
                  <span className="text-xs text-muted-foreground">
                    {t('score')}: {(result.score * 100).toFixed(0)}%
                  </span>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <p className="line-clamp-3 text-sm text-muted-foreground">
                {result.snippet}
              </p>
            </CardContent>
          </Card>
        </Link>
      ))}
    </div>
  );
}
