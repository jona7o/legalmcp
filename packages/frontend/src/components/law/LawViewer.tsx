'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ExternalLink, Calendar } from 'lucide-react';
import type { LawDocument } from '@/types';
import { formatDate } from '@/lib/utils';

interface LawViewerProps {
  law: LawDocument;
}

export function LawViewer({ law }: LawViewerProps) {
  return (
    <div className="space-y-6">
      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between gap-4">
            <div>
              <CardTitle className="text-3xl">{law.title}</CardTitle>
              {law.abbreviation && (
                <p className="mt-2 text-lg text-muted-foreground">
                  {law.abbreviation}
                </p>
              )}
            </div>
            <Badge variant="outline" className="text-lg px-4 py-2">
              {law.jurisdiction}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
            {law.metadata.promulgationDate && (
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4" />
                <span>Verkündet: {formatDate(law.metadata.promulgationDate)}</span>
              </div>
            )}
            {law.metadata.lastModified && (
              <div className="flex items-center gap-2">
                <Calendar className="h-4 w-4" />
                <span>
                  Letzte Änderung: {formatDate(law.metadata.lastModified)}
                </span>
              </div>
            )}
          </div>

          {law.metadata.citation && (
            <div>
              <strong className="text-sm">Fundstelle:</strong>
              <p className="mt-1 text-sm text-muted-foreground">
                {law.metadata.citation}
              </p>
            </div>
          )}

          {law.metadata.url && (
            <a
              href={law.metadata.url}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-sm text-primary hover:underline"
            >
              <ExternalLink className="h-4 w-4" />
              Originalquelle ansehen
            </a>
          )}
        </CardContent>
      </Card>

      {/* Content Tabs */}
      <Tabs defaultValue="full" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="full">Volltext</TabsTrigger>
          <TabsTrigger value="sections">Gliederung</TabsTrigger>
        </TabsList>

        <TabsContent value="full" className="mt-6">
          <Card>
            <CardContent className="pt-6">
              <div
                className="prose prose-sm max-w-none dark:prose-invert"
                dangerouslySetInnerHTML={{ __html: law.content }}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="sections" className="mt-6">
          <div className="space-y-4">
            {law.sections.map((section) => (
              <SectionCard key={section.id} section={section} />
            ))}
          </div>
        </TabsContent>
      </Tabs>

      {/* Disclaimer */}
      <Card className="border-yellow-500/50 bg-yellow-50 dark:bg-yellow-950/20">
        <CardContent className="pt-6">
          <p className="text-sm text-muted-foreground">
            <strong>Hinweis:</strong> Die dargestellten Rechtstexte dienen 
            ausschließlich der Information und stellen keine verbindliche 
            Auskunft dar. Für rechtsverbindliche Auskünfte wenden Sie sich 
            bitte an die zuständigen Behörden oder einen Rechtsanwalt.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

function SectionCard({ section }: { section: any }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">
          {section.number} {section.title}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div
          className="text-sm"
          dangerouslySetInnerHTML={{ __html: section.content }}
        />
        
        {section.subsections && section.subsections.length > 0 && (
          <>
            <Separator />
            <div className="space-y-3 pl-4">
              {section.subsections.map((subsection: any) => (
                <div key={subsection.id}>
                  <h4 className="font-semibold text-sm">
                    {subsection.number} {subsection.title}
                  </h4>
                  <div
                    className="mt-1 text-sm text-muted-foreground"
                    dangerouslySetInnerHTML={{ __html: subsection.content }}
                  />
                </div>
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
