'use client';

import Link from 'next/link';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { FileText, Gavel } from 'lucide-react';
import type { SearchResult } from '@/types';

interface ResultListProps {
  results: SearchResult[];
  isLoading?: boolean;
  error?: string | null;
}

export function ResultList({ results, isLoading, error }: ResultListProps) {
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
            <strong>Fehler:</strong> {error}
          </p>
        </CardContent>
      </Card>
    );
  }

  if (results.length === 0) {
    return (
      <Card>
        <CardContent className="pt-6">
          <p className="text-center text-muted-foreground">
            Keine Ergebnisse gefunden. Versuchen Sie andere Suchbegriffe.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {results.map((result) => (
        <Link
          key={result.id}
          href={result.type === 'law' ? `/law/${result.id}` : `/case/${result.id}`}
        >
          <Card className="transition-shadow hover:shadow-md">
            <CardHeader>
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-start gap-3 flex-1">
                  {result.type === 'law' ? (
                    <FileText className="mt-1 h-5 w-5 text-primary" />
                  ) : (
                    <Gavel className="mt-1 h-5 w-5 text-primary" />
                  )}
                  <div className="flex-1">
                    <h3 className="font-semibold leading-tight">
                      {result.title}
                    </h3>
                    {result.metadata?.abbreviation && (
                      <p className="mt-1 text-sm text-muted-foreground">
                        {result.metadata.abbreviation}
                      </p>
                    )}
                    {result.metadata?.court && result.metadata?.date && (
                      <p className="mt-1 text-sm text-muted-foreground">
                        {result.metadata.court} • {result.metadata.date}
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex gap-2">
                  <Badge variant="outline">{result.jurisdiction}</Badge>
                  <Badge variant={result.type === 'law' ? 'default' : 'secondary'}>
                    {result.type === 'law' ? 'Gesetz' : 'Urteil'}
                  </Badge>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <p
                className="line-clamp-3 text-sm text-muted-foreground"
                dangerouslySetInnerHTML={{ __html: result.snippet }}
              />
            </CardContent>
          </Card>
        </Link>
      ))}
    </div>
  );
}
