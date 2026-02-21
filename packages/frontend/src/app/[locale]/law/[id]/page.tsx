import { notFound } from 'next/navigation';
import { ArrowLeft, ExternalLink, Calendar } from 'lucide-react';
import Link from 'next/link';
import { getTranslations } from 'next-intl/server';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { LawViewer } from '@/components/law/LawViewer';
import { getDocument } from '@/lib/api';

interface LawPageProps {
  params: Promise<{
    id: string;
    locale: string;
  }>;
}

export default async function LawPage({ params }: LawPageProps) {
  const { id, locale } = await params;
  const t = await getTranslations({ locale, namespace: 'law' });

  let document;
  try {
    document = await getDocument(id);
  } catch {
    notFound();
  }

  return (
    <div className="space-y-6">
      <div>
        <Link href="/">
          <Button variant="ghost" size="sm" className="gap-2">
            <ArrowLeft className="h-4 w-4" />
            {t('backToSearch')}
          </Button>
        </Link>
      </div>

      <LawViewer document={document} t={t} />
    </div>
  );
}
