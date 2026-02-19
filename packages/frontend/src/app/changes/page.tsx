'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Calendar, FileText } from 'lucide-react';
import Link from 'next/link';
import { apiClient } from '@/lib/api';
import { formatDate } from '@/lib/utils';
import type { Change, ApiError } from '@/types';

export default function ChangesPage() {
  const [changes, setChanges] = useState<Change[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<string>('all');

  useEffect(() => {
    const loadChanges = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const jurisdiction = activeTab === 'all' ? undefined : activeTab;
        const data = await apiClient.getChanges({ 
          jurisdiction,
          limit: 50 
        });
        setChanges(data);
      } catch (err) {
        const apiError = err as ApiError;
        setError(
          apiError.details || apiError.error || 'Fehler beim Laden der Änderungen'
        );
      } finally {
        setIsLoading(false);
      }
    };

    loadChanges();
  }, [activeTab]);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-4xl font-bold tracking-tight">
          Aktuelle Änderungen
        </h1>
        <p className="mt-2 text-lg text-muted-foreground">
          Übersicht über kürzlich geänderte Gesetze und Verordnungen
        </p>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="grid w-full max-w-md grid-cols-4">
          <TabsTrigger value="all">Alle</TabsTrigger>
          <TabsTrigger value="DE">Deutschland</TabsTrigger>
          <TabsTrigger value="BY">Bayern</TabsTrigger>
          <TabsTrigger value="EU">EU</TabsTrigger>
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
          ) : changes.length === 0 ? (
            <Card>
              <CardContent className="pt-6">
                <p className="text-center text-muted-foreground">
                  Keine Änderungen gefunden.
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {changes.map((change) => (
                <Link key={change.id} href={`/law/${change.lawId}`}>
                  <Card className="transition-shadow hover:shadow-md">
                    <CardHeader>
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex items-start gap-3 flex-1">
                          <FileText className="mt-1 h-5 w-5 text-primary" />
                          <div>
                            <CardTitle className="text-lg">
                              {change.lawTitle}
                            </CardTitle>
                            <div className="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
                              <Calendar className="h-4 w-4" />
                              <span>
                                Geändert am {formatDate(change.changeDate)}
                              </span>
                            </div>
                          </div>
                        </div>
                        <Badge variant="outline">
                          {change.jurisdiction}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent className="space-y-3">
                      <p className="text-sm">{change.description}</p>
                      
                      {change.affectedSections.length > 0 && (
                        <div>
                          <p className="text-sm font-semibold">
                            Betroffene Abschnitte:
                          </p>
                          <div className="mt-1 flex flex-wrap gap-2">
                            {change.affectedSections.map((section, idx) => (
                              <Badge key={idx} variant="secondary">
                                {section}
                              </Badge>
                            ))}
                          </div>
                        </div>
                      )}
                    </CardContent>
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
