import { useTranslations } from 'next-intl';
import { Separator } from '@/components/ui/separator';

export function Footer() {
  const t = useTranslations('footer');

  return (
    <footer className="mt-auto border-t bg-background">
      <div className="container py-8">
        <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
          <div>
            <h3 className="mb-3 text-sm font-semibold">Legal MCP</h3>
            <p className="text-sm text-muted-foreground">{t('tagline')}</p>
          </div>

          <div>
            <h3 className="mb-3 text-sm font-semibold">{t('legalNotices')}</h3>
            <p className="text-xs text-muted-foreground">{t('legalNoticesText')}</p>
          </div>

          <div>
            <h3 className="mb-3 text-sm font-semibold">{t('disclaimer')}</h3>
            <p className="text-xs text-muted-foreground">{t('disclaimerText')}</p>
          </div>
        </div>

        <Separator className="my-6" />

        <div className="flex flex-col gap-2 text-center text-xs text-muted-foreground md:flex-row md:justify-between">
          <p>{t('copyright')}</p>
          <p>{t('noAdvice')}</p>
        </div>
      </div>
    </footer>
  );
}
