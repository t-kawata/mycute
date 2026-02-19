import { defineBoot } from '#q-app/wrappers';
import { i18n } from 'src/i18n/instance';

export default defineBoot(({ app }) => {
  // Set i18n instance on app
  app.use(i18n);
});
