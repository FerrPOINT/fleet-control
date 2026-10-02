---
name: domain-modeling
description: Уточнять терминологию, владельцев данных, инварианты и bounded contexts назначенного продукта.
---

# Модель предметной области

Составить glossary actor/object/action. Назвать владельца каждого поля, aggregate
и source of truth; отделить пользовательский статус от технического cursor/run.
Проверить существующие capabilities прежде нового слоя. Описать инварианты,
revision boundaries, ссылки и права. Dependency направлена от dependent к
dependency; impact читается по входящим ссылкам старой revision. Не переписывать
pinned history, audit или опубликованные evidence. Не превращать proposal в факт
реализации или shared database в межпродуктовое владение данными.
