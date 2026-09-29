WITH RecentPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.OwnerUserId,
        P.Score,
        P.ViewCount,
        P.AnswerCount,
        P.CommentCount,
        P.ClosedDate,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
    FROM 
        Posts P
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
),
UserScores AS (
    SELECT 
        U.Id AS UserId,
        COALESCE(SUM(VB.BountyAmount), 0) AS TotalBounties,
        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVotes,
        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes
    FROM 
        Users U
    LEFT JOIN 
        Votes V ON U.Id = V.UserId
    LEFT JOIN 
        Votes VB ON U.Id = VB.UserId AND VB.VoteTypeId = 8
    GROUP BY 
        U.Id
),
TopUsers AS (
    SELECT 
        U.Id,
        U.DisplayName,
        US.TotalBounties,
        (US.UpVotes - US.DownVotes) AS NetVotes
    FROM 
        Users U
    JOIN 
        UserScores US ON U.Id = US.UserId
    WHERE 
        U.Reputation > 100
    ORDER BY 
        NetVotes DESC, U.Id
    LIMIT 10
)
SELECT 
    RP.Title,
    RP.CreationDate,
    RP.ViewCount,
    T.DisplayName AS OwnerName,
    TU.TotalBounties,
    TU.NetVotes
FROM 
    RecentPosts RP
LEFT JOIN 
    Users T ON RP.OwnerUserId = T.Id
LEFT JOIN 
    TopUsers TU ON T.Id = TU.Id
WHERE 
    RP.ClosedDate IS NULL OR RP.ClosedDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days'
ORDER BY 
    RP.Score DESC, RP.ViewCount DESC;