'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from 'next-intl';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Calendar, FileText } from 'lucide-react';
import Link from 'next/link';
import { getChanges } from '@/lib/api';
import type { DocumentVersion, ApiError } from '@/types';

function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

export default function ChangesPage() {
  const t = useTranslations('changes');

  const [versions, setVersions] = useState<DocumentVersion[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<string>('all');

  useEffect(() => {
    const loadChanges = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const jurisdiction = activeTab === 'all' ? undefined : activeTab;
        const data = await getChanges({
          jurisdiction,
          limit: 50,
        });
        setVersions(data.results);
      } catch (err) {
        const apiError = err as ApiError;
        setError(
          apiError.detail || apiError.details || apiError.error || t('loadError')
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadChanges();
  }, [activeTab, t]);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-4xl font-bold tracking-tight">{t('title')}</h1>
        <p className="mt-2 text-lg text-muted-foreground">{t('subtitle')}</p>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="grid w-full max-w-lg grid-cols-6">
          <TabsTrigger value="all">{t('all')}</TabsTrigger>
          <TabsTrigger value="DE">{t('de')}</TabsTrigger>
          <TabsTrigger value="EU">{t('eu')}</TabsTrigger>
          <TabsTrigger value="FR">{t('fr')}</TabsTrigger>
          <TabsTrigger value="IT">{t('it')}</TabsTrigger>
          <TabsTrigger value="ES">{t('es')}</TabsTrigger>
        </TabsList>

        <TabsContent value={activeTab} className="mt-6">
          {isLoading ? (
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
          ) : error ? (
            <Card className="border-destructive">
              <CardContent className="pt-6">
                <p className="text-sm text-destructive">
                  <strong>Fehler:</strong> {error}
                </p>
              </CardContent>
            </Card>
          ) : versions.length === 0 ? (
            <Card>
              <CardContent className="pt-6">
                <p className="text-center text-muted-foreground">{t('noChanges')}</p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {versions.map((version) => (
                <Link key={version.id} href={`/law/${version.document_id}`}>
                  <Card className="transition-shadow hover:shadow-md">
                    <CardHeader>
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex items-start gap-3 flex-1">
                          <FileText className="mt-1 h-5 w-5 text-primary" />
                          <div>
                            <CardTitle className="text-lg">
                              {version.document_id}
                            </CardTitle>
                            <div className="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
                              <Calendar className="h-4 w-4" />
                              <span>
                                {t('changedOn')} {formatDate(version.changed_at)}
                              </span>
                            </div>
                          </div>
                        </div>
                        <Badge variant="outline">v{version.version_number}</Badge>
                      </div>
                    </CardHeader>
                    {version.change_summary && (
                      <CardContent>
                        <p className="text-sm">{version.change_summary}</p>
                      </CardContent>
                    )}
                  </Card>
                </Link>
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
