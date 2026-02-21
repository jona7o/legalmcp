// Root layout — required by Next.js.
// next-intl middleware redirects all root requests to /[locale]/ automatically.
// This layout is only rendered for paths that the middleware does not intercept
// (e.g. /api routes). Keep it minimal.
export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html>
      <body>{children}</body>
    </html>
  );
}
