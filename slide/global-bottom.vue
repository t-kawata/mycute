<script setup lang="ts">
import { useNav } from '@slidev/client'
const { currentPage, total } = useNav()
</script>

<template>
  <footer class="absolute bottom-0 right-0 p-2 text-xs text-gray-400">
    {{ currentPage }} / {{ total }}
  </footer>
</template>
