WITH UserBadges AS (
    SELECT 
        U.Id AS UserId,
        COUNT(B.Id) AS BadgeCount,
        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM 
        Users U
    LEFT JOIN 
        Badges B ON U.Id = B.UserId
    GROUP BY 
        U.Id
),
RecentPosts AS (
    SELECT 
        P.Id AS PostId,
        P.OwnerUserId,
        P.Title,
        P.CreationDate,
        P.Score,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS RecentPostRank
    FROM 
        Posts P
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
),
AggregateVotes AS (
    SELECT 
        V.PostId,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        COUNT(*) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes
    FROM 
        Votes V
    GROUP BY 
        V.PostId
)
SELECT 
    U.DisplayName,
    U.Reputation,
    UB.BadgeCount,
    UB.GoldBadges,
    UB.SilverBadges,
    UB.BronzeBadges,
    RP.PostId,
    RP.Title,
    RP.CreationDate,
    COALESCE(AV.UpVotes, 0) AS UpVotes,
    COALESCE(AV.DownVotes, 0) AS DownVotes,
    CASE 
        WHEN UB.BadgeCount > 10 THEN 'Veteran'
        WHEN UB.BadgeCount > 5 THEN 'Experienced'
        ELSE 'Novice'
    END AS UserLevel
FROM 
    Users U
JOIN 
    UserBadges UB ON U.Id = UB.UserId
LEFT JOIN 
    RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentPostRank = 1
LEFT JOIN 
    AggregateVotes AV ON RP.PostId = AV.PostId
WHERE 
    U.Reputation > 50
ORDER BY 
    U.Reputation DESC, 
    RP.CreationDate DESC
LIMIT 100;