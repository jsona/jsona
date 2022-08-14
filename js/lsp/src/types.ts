export namespace Server {
  interface ServerNotifications {
    "jsona/messageWithOutput": {
      params: {
        kind: "info" | "warn" | "error";
        message: string;
      };
    };
    "jsona/initializeWorkspace": {
      params: {
        rootUri: string
      };
    };
  }

  export type NotificationMethod = keyof ServerNotifications;

  export type NotificationParams<T extends keyof ServerNotifications> =
    ServerNotifications[T] extends NotificationDescription
      ? ServerNotifications[T]["params"]
      : never;
}

export namespace Client {
  interface ClientNotifications {
    "jsona/associateSchemas": {
      params: {
        associations: AssociateSchema[]
      };
    };
  }

  interface ClientRequests {
    "jsona/listSchemas": {
      params: {
        documentUri: string;
      };
      response: {
        schemas: Array<SchemaInfo>;
      };
    };
    "jsona/associatedSchema": {
      params: {
        documentUri: string;
      };
      response: {
        schema?: SchemaInfo | null;
      };
    };
  }

  export type NotificationMethod = keyof ClientNotifications;

  export type NotificationParams<T extends keyof ClientNotifications> =
    ClientNotifications[T] extends NotificationDescription
      ? ClientNotifications[T]["params"]
      : never;

  export type RequestMethod = keyof ClientRequests;

  export type RequestParams<T extends keyof ClientRequests> =
    ClientRequests[T] extends RequestDescription
      ? ClientRequests[T]["params"]
      : never;

  export type RequestResponse<T extends keyof ClientRequests> =
    ClientRequests[T] extends RequestDescription
      ? ClientRequests[T]["response"]
      : never;
}

interface NotificationDescription {
  readonly params: any;
}

interface RequestDescription {
  readonly params: any;
  readonly response: any;
}

export type AssociationRule =
  | { glob: string }
  | { regex: string }
  | { url: string };

export interface SchemaInfo {
  url: string;
  meta: any;
}

export interface AssociateSchema {
  schemaUri: string;
  rule: AssociationRule;
  meta?: any;
}