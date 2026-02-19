<template>
  <q-page class="bg-grey-2 q-pa-md">
    <div class="text-h5 q-mb-md">Settings</div>
    <q-list bordered separator class="bg-white rounded-borders">
      <q-item clickable v-ripple>
        <q-item-section avatar><q-icon name="wifi" color="primary" /></q-item-section>
        <q-item-section>Wi-Fi</q-item-section>
        <q-item-section side><q-badge color="green">Connected</q-badge></q-item-section>
      </q-item>
      <q-item clickable v-ripple>
        <q-item-section avatar><q-icon name="bluetooth" color="blue" /></q-item-section>
        <q-item-section>Bluetooth</q-item-section>
        <q-item-section side>On</q-item-section>
      </q-item>
      <q-item clickable v-ripple>
        <q-item-section avatar><q-icon name="notifications" color="orange" /></q-item-section>
        <q-item-section>Notifications</q-item-section>
      </q-item>
      <q-item clickable v-ripple>
        <q-item-section avatar><q-icon name="security" color="red" /></q-item-section>
        <q-item-section>Privacy & Security</q-item-section>
      </q-item>
    </q-list>
  </q-page>
</template>

<script setup lang="ts">
// Dummy Settings App
</script>

<style scoped>
.rounded-borders {
  border-radius: 12px;
}
</style>
